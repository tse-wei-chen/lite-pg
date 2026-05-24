mod app;
mod db;
mod export;
mod history;
mod modes;
mod tui;

use app::{Action, App};
use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::style::{Color, Style};
use ratatui::Terminal;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};
use std::io::stdout;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(name = "lite-pg", version, about = "Lightning-fast PostgreSQL TUI client")]
struct Cli {
    #[arg(long, default_value = "localhost")]
    host: String,

    #[arg(long, default_value_t = 5432)]
    port: u16,

    #[arg(long, default_value = "postgres")]
    user: String,

    #[arg(long)]
    password: Option<String>,

    #[arg(long, default_value = "postgres")]
    dbname: String,

    #[arg(long)]
    conn: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run(&mut terminal, &args).await;

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

async fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, args: &Cli) -> anyhow::Result<()>
{
    let mut app = App::new();

    // Connect to database
    let _conn_str = args.conn.clone().unwrap_or_else(|| {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            args.user,
            args.password.as_deref().unwrap_or(""),
            args.host,
            args.port,
            args.dbname
        )
    });

    match db::connect(
        &args.host,
        args.port,
        &args.user,
        args.password.as_deref(),
        &args.dbname,
    )
    .await
    {
        Ok(pool) => {
            app.set_db(pool);
            app.refresh_schema().await;
        }
        Err(e) => {
            app.results.error = Some(format!("Connection failed: {e}"));
        }
    }

    // Event loop
    let mut last_kc: Option<(KeyCode, Instant)> = None;
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if app.quit {
            break;
        }

        if !crossterm::event::poll(Duration::from_millis(50))? {
            continue;
        }

        let event = crossterm::event::read()?;
        if let Event::Key(key) = event {
            // Windows console sends both KeyPress and KeyRelease for every
            // physical keypress — skip Release to avoid double-processing.
            if key.kind == KeyEventKind::Release {
                continue;
            }

            // Some keys (Tab, Enter) also fire twin-pair VKey + Char events
            // on Windows — skip if this is the paired counterpart.
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

            let action = app.handle_key(key);

            match action {
                Action::Quit => break,
                Action::ExecuteQuery(sql) => {
                    app.run_query(&sql).await;
                }
                Action::ExecuteExplain(sql) => {
                    app.run_explain(&sql).await;
                }
                Action::None => {}
            }
        }
    }

    Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = tui::layout::main_layout(f.area());

    // Status bar
    tui::status_bar::render(f, chunks[0], &app.mode, &app.focus, &app.elapsed, app.connected);

    // Schema tree
    tui::schema_tree::render(
        f,
        chunks[1],
        &app.tables,
        &mut app.schema_state,
        &app.schema_expanded,
    );

    // Query input
    tui::query_input::render(f, chunks[2], &mut app.query_input);

    // Results
    let result_area = chunks[3];
    if app.show_history {
        render_history_panel(f, result_area, app);
    } else {
        tui::results_table::render(
            f,
            result_area,
            &app.results,
            &mut app.results_state,
            app.scroll_h,
        );
    }
}

fn render_history_panel(f: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &mut App) {
    let entries = app.history.search(&app.history_search);
    let items: Vec<ratatui::widgets::ListItem> = entries
        .iter()
        .enumerate()
        .map(|(_, entry)| {
            let first_line = entry.sql.lines().next().unwrap_or("");
            let line = Line::from(Span::styled(
                format!(" {}ms | {}",
                    (entry.elapsed_ms as u64),
                    if first_line.len() > 60 {
                        format!("{}...", &first_line[..60])
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
