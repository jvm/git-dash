mod app;
mod discovery;
mod git;
mod logger;
mod status;
mod ui;
mod worker;

use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;

use app::App;
use git::friendly_error;
use logger::{init_logger, log_debug};
use status::git_status;
use ui::render_ui;
use worker::{spawn_worker, Action, WorkerEvent};

const TICK_RATE: Duration = Duration::from_millis(100);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = parse_args()?;
    if config.debug {
        init_logger("git-dash-debug.log")?;
    }
    log_debug("Starting git-dash");
    let root = config.root;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (evt_tx, evt_rx) = mpsc::channel();

    let worker_handle = spawn_worker(cmd_rx, evt_tx);

    let mut app = App::new(root.clone(), cmd_tx);
    app.request_scan();

    let res = run_app(&mut terminal, &mut app, evt_rx);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    app.request_quit();
    let _ = worker_handle.join();

    if let Err(err) = res {
        eprintln!("{err}");
    }

    Ok(())
}

fn parse_args() -> Result<Config, Box<dyn std::error::Error>> {
    let mut root: Option<PathBuf> = None;
    let mut debug = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--debug" | "-d" => {
                debug = true;
            }
            _ if arg.starts_with('-') => {
                return Err(format!("Unknown option: {arg}").into());
            }
            _ => {
                if root.is_some() {
                    return Err("Too many arguments".into());
                }
                root = Some(PathBuf::from(arg));
            }
        }
    }

    Ok(Config {
        root: root.unwrap_or(std::env::current_dir()?),
        debug,
    })
}

fn print_help() {
    println!(
        "git-dash\nA fast TUI dashboard for discovering and managing multiple Git repositories.\n\nUSAGE:\n    git-dash [path]\n\nARGS:\n    path    Optional directory to scan (defaults to current directory)\n\nOPTIONS:\n    -d, --debug    Enable debug logging to git-dash-debug.log\n    -h, --help     Print help information"
    );
}

struct Config {
    root: PathBuf,
    debug: bool,
}

fn run_app(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    app: &mut App,
    evt_rx: mpsc::Receiver<WorkerEvent>,
) -> io::Result<()> {
    let mut last_tick = Instant::now();

    loop {
        drain_worker_events(app, &evt_rx);
        terminal.draw(|frame| render_ui(frame, app))?;

        if app.should_quit {
            return Ok(());
        }

        let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                handle_key_event(app, key);
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            last_tick = Instant::now();
        }
    }
}

fn handle_key_event(app: &mut App, key: KeyEvent) {
    // Help screen takes priority - close it with any key
    if app.help_visible {
        app.toggle_help();
        return;
    }

    // Search mode takes priority
    if app.search_mode {
        handle_search_key(app, key);
        return;
    }

    if app.confirmation.is_some() {
        handle_confirm_key(app, key);
        return;
    }

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('r') => app.request_refresh(),
        KeyCode::Char('p') => app.request_confirm(Action::Pull),
        KeyCode::Char('u') => app.request_confirm(Action::Push),
        KeyCode::Char('s') => app.cycle_sort_order(),
        KeyCode::Char('?') => app.toggle_help(),
        KeyCode::Char('/') => app.enter_search_mode(),
        KeyCode::Esc => app.exit_search_mode(),
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        KeyCode::PageDown => app.page_down(),
        KeyCode::PageUp => app.page_up(),
        KeyCode::Home | KeyCode::Char('g') => app.jump_to_first(),
        KeyCode::End | KeyCode::Char('G') => app.jump_to_last(),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true
        }
        _ => {}
    }
}

fn handle_confirm_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') => {
            if let Some(action) = app.confirmation.take() {
                app.perform_action(action);
            }
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.set_status("Action canceled".to_string());
            app.confirmation = None;
        }
        _ => {}
    }
}

fn handle_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) => app.search_push_char(c),
        KeyCode::Backspace => app.search_pop_char(),
        KeyCode::Enter | KeyCode::Esc => {
            app.search_mode = false;
        }
        _ => {}
    }
}

fn drain_worker_events(app: &mut App, evt_rx: &mpsc::Receiver<WorkerEvent>) {
    while let Ok(event) = evt_rx.try_recv() {
        match event {
            WorkerEvent::ScanComplete(repos) => {
                app.repos = repos;
                app.sort_repos();
                app.loading = false;
                app.scan_progress = 1.0;
                app.set_status("Scan complete".to_string());
            }
            WorkerEvent::RefreshComplete(repos) => {
                app.repos = repos;
                app.sort_repos();
                app.set_status("Status refreshed".to_string());
            }
            WorkerEvent::ScanProgress { ratio } => {
                app.scan_progress = ratio;
            }
            WorkerEvent::ActionResult {
                path,
                action,
                result,
            } => {
                let action_label = match action {
                    Action::Pull => "Pull",
                    Action::Push => "Push",
                };
                match result {
                    Ok(message) => app.set_status(format!("{action_label} OK: {message}")),
                    Err(message) => {
                        let friendly_msg = friendly_error(&message);
                        app.set_status(format!("{action_label} failed: {friendly_msg}"))
                    }
                }
                if let Some(repo) = app.repos.iter_mut().find(|repo| repo.path == path) {
                    if let Ok(status) = git_status(&path, &repo.git_dir) {
                        *repo = status;
                    }
                }
            }
        }
    }
}
