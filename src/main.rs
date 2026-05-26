mod app;
mod bookmarks;
mod connections;
mod db;
mod export;
mod history;
mod modes;
mod tui;
mod util;

use app::{Action, App, Page};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};
use ratatui::Terminal;
use std::io::stdout;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run(&mut terminal).await;

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

async fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    let mut app = App::new();

    let mut last_kc: Option<(KeyCode, Instant)> = None;
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if app.quit {
            break;
        }

        if !crossterm::event::poll(Duration::from_millis(50))? {
            // Prefetch column data in one batch query
            if !app.prefetched {
                app.run_prefetch().await;
            }

            // Search debounce
            if app.current_page == Page::Search && app.search_pending {
                if let Some(t) = app.last_search_time {
                    if t.elapsed() > Duration::from_millis(200) && app.search_query.len() >= 2 {
                        app.search_pending = false;
                        let q = app.search_query.clone();
                        app.run_search(&q).await;
                    }
                }
            }
            continue;
        }

        let event = crossterm::event::read()?;
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Release {
                continue;
            }

            let now = Instant::now();
            let dup = last_kc.as_ref().is_some_and(|(code, t)| {
                t.elapsed() < Duration::from_millis(50)
                    && matches!((code, key.code),
                        (KeyCode::Tab, KeyCode::Char('\t'))
                        | (KeyCode::Char('\t'), KeyCode::Tab)
                        | (KeyCode::Enter, KeyCode::Char('\r'))
                        | (KeyCode::Char('\r'), KeyCode::Enter)
                        | (KeyCode::Enter, KeyCode::Char('\n'))
                        | (KeyCode::Char('\n'), KeyCode::Enter)
                        | (KeyCode::Char('\r'), KeyCode::Char('\n'))
                        | (KeyCode::Char('\n'), KeyCode::Char('\r'))
                    )
            });
            if dup {
                continue;
            }
            last_kc = Some((key.code, now));

            // ── Menu overlay ───────────────────────────────────
            if app.menu.show {
                match key.code {
                    KeyCode::Esc => {
                        if app.menu.level > 0 {
                            app.menu.level = 0;
                            app.menu.selection = app.menu.parent;
                        } else {
                            app.menu.show = false;
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let max = if app.menu.level == 0 {
                            8
                        } else if app.menu.parent == 4 {
                            5
                        } else if app.menu.parent == 6 {
                            4
                        } else {
                            0
                        };
                        app.menu.selection = (app.menu.selection + 1).min(max);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.menu.selection = app.menu.selection.saturating_sub(1);
                    }
                    KeyCode::Enter => {
                        if app.menu.level == 0 {
                            match app.menu.selection {
                                0 => {
                                    app.current_page = Page::Home;
                                    app.menu.show = false;
                                }
                                1 => {
                                    app.current_page = Page::Dashboard;
                                    app.menu.show = false;
                                    app.refresh_dashboard().await;
                                }
                                2 => {
                                    app.current_page = Page::Bookmarks;
                                    app.menu.show = false;
                                }
                                3 => {
                                    app.current_page = Page::Search;
                                    app.menu.show = false;
                                }
                                4 => {
                                    app.menu.level = 1;
                                    app.menu.selection = 0;
                                    app.menu.parent = 4;
                                }
                                5 => {
                                    app.menu.level = 1;
                                    app.menu.selection = 0;
                                    app.menu.parent = 5;
                                }
                                6 => {
                                    app.menu.level = 1;
                                    app.menu.selection = 0;
                                    app.menu.parent = 6;
                                }
                                7 => {
                                    app.current_page = Page::Help;
                                    app.menu.show = false;
                                }
                                8 => {
                                    app.quit = true;
                                }
                                _ => {}
                            }
                        } else if app.menu.parent == 4 {
                            // Management submenu
                            app.menu.show = false;
                            app.menu.level = 0;
                            match app.menu.selection {
                                0 => { app.current_page = Page::Roles; app.refresh_roles().await; }
                                1 => { app.current_page = Page::Databases; app.refresh_databases().await; }
                                2 => { app.current_page = Page::Functions; app.refresh_functions().await; }
                                3 => { app.current_page = Page::Extensions; app.refresh_extensions().await; }
                                4 => { app.current_page = Page::Settings; app.refresh_settings().await; }
                                5 => { app.current_page = Page::Replication; app.refresh_replication().await; }
                                _ => {}
                            }
                        } else if app.menu.parent == 6 {
                            // Export submenu
                            app.menu.show = false;
                            app.menu.level = 0;
                            match app.menu.selection {
                                0 => { app.export_csv(); }
                                1 => { app.export_json(); }
                                2 => { app.export_sql_insert(); }
                                3 => { app.save_markdown_to_file(); }
                                4 => { app.copy_markdown(); }
                                _ => {}
                            }
                        } else {
                            // Connection submenu
                            app.menu.show = false;
                            app.menu.level = 0;
                            match app.menu.selection {
                                0 => { app.current_page = Page::ConnectionManager; }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }

            // ── ESC → toggle menu (not when confirmation is active) ─
            if key.code == KeyCode::Esc && app.menu.level == 0 && !app.show_connection_form && !app.show_role_form && app.confirm_message.is_none() && app.mode != app::Mode::Insert && app.mode != app::Mode::PropertyEditor {
                app.menu.show = !app.menu.show;
                app.menu.selection = 0;
                continue;
            }

            let action = app.handle_key(key);

            match action {
                Action::Quit => break,
                Action::Connect(idx) => {
                    connect_to_profile(&mut app, idx).await;
                }
                Action::RefreshSchema => {
                    app.refresh_schema().await;
                }
                Action::ShowDetail(si, oi) => {
                    app.show_object_detail(si, oi).await;
                }
                Action::ExecuteQuery(sql) => {
                    app.run_query(&sql).await;
                }
                Action::ExecuteExplain(sql) => {
                    app.run_explain(&sql).await;
                }
                Action::ShowDdl(si, oi) => {
                    app.show_ddl(si, oi).await;
                }
                Action::NextPage => {
                    app.next_page();
                    app.load_browse_page().await;
                }
                Action::PrevPage => {
                    app.prev_page();
                    app.load_browse_page().await;
                }
                Action::ExecuteConfirm => {
                    if let Some(sql) = app.confirm_sql.take() {
                        app.confirm_message = None;
                        app.property_editor = None;
                        app.run_query(&sql).await;
                    }
                }
                Action::CancelConfirm => {
                    app.confirm_message = None;
                    app.confirm_sql = None;
                }
                Action::None => {}
                Action::GenerateSelect(si, oi) => {
                    app.generate_select(si, oi);
                    let sql = app.current_tab().lines().join("\n");
                    app.run_query(&sql).await;
                }
                Action::GenerateInsert(si, oi) => {
                    app.generate_insert(si, oi);
                }
                Action::GenerateUpdate(si, oi) => {
                    app.generate_update(si, oi);
                }
                Action::RefreshDashboard => {
                    app.refresh_dashboard().await;
                }
                Action::RunSearch(query) => { app.run_search(&query).await; }
                Action::SwitchDatabase(dbname) => {
                    switch_database(&mut app, &dbname).await;
                }
            }
        }
    }

    Ok(())
}

async fn switch_database(app: &mut App, dbname: &str) {
    let idx = match app.connections.active_index {
        Some(i) => i,
        None => {
            app.results = db::QueryResult {
                error: Some("No active connection profile to switch database".to_string()),
                ..Default::default()
            };
            return;
        }
    };
    let password = app.connections.get_decrypted_password(idx);
    let profile = match app.connections.profiles.get(idx) {
        Some(p) => p.clone(),
        None => return,
    };

    match db::connect(&profile.host, profile.port, &profile.user, password.as_deref(), dbname, &profile.ssl_mode).await {
        Ok(pool) => {
            let name = format!("{} ({})", profile.name, dbname);
            app.set_db(pool, name);
            app.current_page = Page::Home;
            app.focus = app::Focus::Schema;
            app.refresh_schema().await;
        }
        Err(e) => {
            app.results = db::QueryResult {
                error: Some(format!("Switch database failed: {e}")),
                ..Default::default()
            };
        }
    }
}

async fn connect_to_profile(app: &mut App, index: usize) {
    let password = app.connections.get_decrypted_password(index);
    let profile = match app.connections.profiles.get(index) {
        Some(p) => p.clone(),
        None => return,
    };

    match db::connect(&profile.host, profile.port, &profile.user, password.as_deref(), &profile.dbname, &profile.ssl_mode).await {
        Ok(pool) => {
            let name = profile.name.clone();
            app.set_db(pool, name);
            app.connections.set_active(index);
            app.focus = app::Focus::Schema;
            app.refresh_schema().await;
        }
        Err(e) => {
            app.results = db::QueryResult {
                error: Some(format!("Connection failed: {e}")),
                ..Default::default()
            };
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let is_overlay = render_overlays(f, app);

    if !is_overlay {
        match app.current_page {
            Page::Home => render_home(f, app),
            Page::Dashboard => render_page_full(f, app, |f, app| {
                tui::dashboard::render(
                    f, f.area(),
                    app.dashboard_tab,
                    &app.server_overview,
                    &app.db_stats,
                    &app.active_queries,
                    &app.table_stats,
                    &mut app.dashboard_list_state,
                    &app.dashboard_error,
                );
            }),
            Page::Roles => render_page_full(f, app, |f, app| {
                let confirm = app.role_confirm.as_ref().map(|n| format!("Drop role '{}'?", n));
                tui::role_panel::render(f, f.area(), &app.roles, &mut app.role_list_state, confirm.as_deref());
            }),
            Page::Databases => render_page_full(f, app, |f, app| {
                tui::database_panel::render(f, f.area(), &app.databases, &mut app.database_list_state);
            }),
            Page::Functions => render_page_full(f, app, |f, app| {
                tui::function_panel::render(f, f.area(), &app.functions, &mut app.function_list_state);
            }),
            Page::Extensions => render_page_full(f, app, |f, app| {
                tui::extension_panel::render(f, f.area(), &app.extensions, &mut app.extension_list_state);
            }),
            Page::Settings => render_page_full(f, app, |f, app| {
                tui::settings_panel::render(
                    f, f.area(), &app.settings, &mut app.settings_list_state, &app.settings_filter_category,
                );
            }),
            Page::Replication => render_page_full(f, app, |f, app| {
                if app.replication_tab == 0 {
                    tui::detail_panel::render_replication_pub(
                        f, f.area(), &app.publications, &mut app.replication_list_state,
                    );
                } else {
                    tui::detail_panel::render_replication_sub(
                        f, f.area(), &app.subscriptions, &mut app.replication_list_state,
                    );
                }
            }),
            Page::Search => render_page_full(f, app, |f, app| {
                tui::search_popup::render(
                    f, f.area(), &app.search_query, &app.search_results, &mut app.search_list_state,
                );
            }),
            Page::Bookmarks => render_page_full(f, app, |f, app| {
                let bm = app.bookmark_storage.search("");
                tui::bookmark_panel::render(f, f.area(), &bm, &mut app.bookmark_list_state);
            }),
            Page::ConnectionManager => render_page_full(f, app, |f, app| {
                tui::connection_list::render(f, f.area(), app);
            }),
            Page::Help => render_help(f, app),
        }

        // Shortcut bar for full-page pages (Home handles its own)
        if app.current_page != Page::Home {
            let area = f.area();
            let hint_area = Rect {
                x: 0,
                y: area.height.saturating_sub(1),
                width: area.width,
                height: 1,
            };
            tui::shortcut_bar::render(f, hint_area, app);
        }
    }

    // Confirmation popup (renders on top of everything including overlays)
    if let Some(ref msg) = app.confirm_message {
        tui::confirm_popup::render(f, f.area(), msg);
    }

    // Notification (export success/error, dismissed on next keypress)
    if let Some(ref msg) = app.notification {
        let notif = ratatui::widgets::Paragraph::new(ratatui::text::Line::from(
            ratatui::text::Span::styled(msg, ratatui::style::Style::default().fg(Color::Green)),
        ))
        .block(
            ratatui::widgets::Block::default()
                .title(" Export ")
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(ratatui::style::Style::default().fg(Color::Green)),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });
        let area = ratatui::layout::Rect {
            x: f.area().width.saturating_sub(60).min(f.area().width.saturating_sub(2)) / 2,
            y: f.area().height.saturating_sub(4),
            width: 60.min(f.area().width),
            height: 3,
        };
        f.render_widget(notif, area);
    }

    // Menu overlay on top of everything
    if app.menu.show {
        render_menu(f, app);
    }
}

fn render_overlays(f: &mut ratatui::Frame, app: &mut App) -> bool {
    let area = f.area();
    if let Some(ref editor) = app.property_editor {
        if let Some(obj) = app.property_editor.as_ref().and_then(|e| {
            if e.schema_idx < app.schemas.len() && e.object_idx < app.schemas[e.schema_idx].objects.len() {
                Some(&app.schemas[e.schema_idx].objects[e.object_idx])
            } else { None }
        }) {
            tui::property_editor::render(f, area, editor, obj);
        }
        return true;
    }
    if app.show_connection_form {
        tui::connection_form::render(f, area, app);
        return true;
    }
    if app.show_role_form {
        tui::role_form::render(f, area, &app.role_form);
        return true;
    }
    false
}

fn render_menu(f: &mut ratatui::Frame, app: &mut App) {
    let area = f.area();
    let popup = ratatui::layout::Rect {
        x: area.width.saturating_sub(68) / 2,
        y: area.height.saturating_sub(16) / 2,
        width: 68.min(area.width),
        height: 16.min(area.height),
    };
    tui::main_menu::render(f, popup, app.menu.level, app.menu.selection, app.menu.parent);
}

fn render_page_full(f: &mut ratatui::Frame, app: &mut App, render_fn: impl FnOnce(&mut ratatui::Frame, &mut App)) {
    render_fn(f, app);
}

fn render_home(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = tui::layout::main_layout(f.area());

    tui::status_bar::render(
        f,
        chunks[0],
        &app.mode,
        &app.focus,
        &app.elapsed,
        app.connected,
        app.current_connection_name.as_ref(),
        app.show_connection_panel,
    );

    tui::schema_tree::render(f, chunks[1], app, app.focus == crate::app::Focus::Schema);

    let query_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(2),
            ratatui::layout::Constraint::Min(1),
        ])
        .split(chunks[2]);
    let tab_area = query_chunks[0];
    let editor_area = query_chunks[1];

    tui::tab_bar::render(
        f,
        tab_area,
        app.active_tab,
        &app.query_tabs,
        app.focus == crate::app::Focus::QueryInput,
    );

    tui::query_input::render(f, editor_area, &mut app.query_tabs[app.active_tab], app.focus == crate::app::Focus::QueryInput);
    if app.completion.visible && app.focus == crate::app::Focus::QueryInput {
        tui::completion_popup::render(f, editor_area, &app.completion);
    }

    let result_area = chunks[3];
    if app.show_history {
        render_history_panel(f, result_area, app);
    } else if app.explain_output.is_some() {
        if let Some(ref explain) = app.explain_output {
                tui::results_table::render(f, result_area, explain, &mut app.results_state, 0, true, &app.visual_selection);
        }
    } else if app.detail_object.is_some() {
        if let Some(ref obj) = app.detail_object {
            tui::detail_panel::render(f, result_area, obj, &mut app.detail_state);
        }
    } else {
        tui::results_table::render(
            f,
            result_area,
            &app.results,
            &mut app.results_state,
            app.scroll_h,
            app.focus == crate::app::Focus::Results,
            &app.visual_selection,
        );
    }

    tui::shortcut_bar::render(f, chunks[4], app);
}

fn render_help(f: &mut ratatui::Frame, _app: &mut App) {
    let area = f.area();
    let block = Block::default()
        .title(" Help (Esc: back) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(" NAVIGATION", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD))),
        Line::from(Span::styled("  Tab          Cycle focus: Schema -> Editor -> Results", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Esc          Open/close main menu", Style::default().fg(Color::White))),
        Line::from(Span::styled("  j/k  or ↓/↑  Move selection in focus", Style::default().fg(Color::White))),
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(" QUERY EDITING", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD))),
        Line::from(Span::styled("  Alt+Enter    Execute current query", Style::default().fg(Color::White))),
        Line::from(Span::styled("  F5           EXPLAIN current query", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Ctrl+T       New tab", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Ctrl+W       Close tab", Style::default().fg(Color::White))),
        Line::from(Span::styled("  ] / [       Next / Previous tab", Style::default().fg(Color::White))),
        Line::from(Span::styled("  i           Enter insert mode (in editor)", Style::default().fg(Color::White))),
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(" SCHEMA TREE", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD))),
        Line::from(Span::styled("  Enter       Expand/collapse object", Style::default().fg(Color::White))),
        Line::from(Span::styled("  o / u       Generate SELECT / UPDATE", Style::default().fg(Color::White))),
        Line::from(Span::styled("  D           Show object DDL", Style::default().fg(Color::White))),
        Line::from(Span::styled("  d / t / v   Drop / Truncate / Vacuum", Style::default().fg(Color::White))),
        Line::from(Span::styled("  r           Refresh schema tree", Style::default().fg(Color::White))),
        Line::from(Span::styled("  n / p       Next / Previous page (browsing)", Style::default().fg(Color::White))),
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(" RESULTS", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD))),
        Line::from(Span::styled("  v           Enter visual mode (select rows)", Style::default().fg(Color::White))),
        Line::from(Span::styled("  h/l         Scroll horizontally", Style::default().fg(Color::White))),
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(" EXPORT (Esc > Export)", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD))),
        Line::from(Span::styled("  Esc > Export > CSV            Export results as CSV file", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Esc > Export > JSON           Export results as JSON file", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Esc > Export > SQL INSERT     Export results as INSERT statements", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Esc > Export > Markdown file  Save results as Markdown file", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Esc > Export > Markdown clipboard  Copy results as Markdown", Style::default().fg(Color::White))),
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(" MENU (Esc)", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD))),
        Line::from(Span::styled("  Dashboard    Server stats, active queries", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Search       Global object search", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Bookmarks    Saved SQL queries", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Management   Roles, Databases, Functions...", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Export       CSV, JSON, SQL INSERT, Markdown", Style::default().fg(Color::White))),
        Line::from(Span::styled("  Connection   Connect, disconnect, manage profiles", Style::default().fg(Color::White))),
    ];

    let paragraph = ratatui::widgets::Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(paragraph, inner);
}

fn render_history_panel(f: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &mut App) {
    let entries = app.history.search(&app.history_search);
    let items: Vec<ratatui::widgets::ListItem> = entries
        .iter()
        .enumerate()
        .map(|(_, entry)| {
            let first_line = entry.sql.lines().next().unwrap_or("");
            let line = Line::from(Span::styled(
                format!(
                    " {}ms | {}",
                    (entry.elapsed_ms as u64),
                    if first_line.chars().count() > 60 {
                        format!("{}...", first_line.chars().take(60).collect::<String>())
                    } else {
                        first_line.to_string()
                    }
                ),
                Style::default().fg(Color::Cyan),
            ));
            ratatui::widgets::ListItem::new(line)
        })
        .collect();

    let list = ratatui::widgets::List::new(items)
        .block(Block::default().title(" History (/ to close) ").borders(Borders::ALL));
    f.render_widget(list, area);
}
