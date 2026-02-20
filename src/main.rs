mod actions;
mod app;
mod config;
mod git;
mod monitor;
mod scanner;
mod setup;
mod ui;

use anyhow::Result;
use app::{App, AppMode};
use chrono::Local;
use clap::Parser;
use config::default_config_path;
use crossterm::{
    event::{Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use git::Repo;
use monitor::StatusCache;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::Sender;

#[derive(Parser, Debug)]
#[command(name = "gitpulse", about = "Monitor all your git repos from one TUI")]
struct Cli {
    /// Path to config file (default: ~/.config/gitpulse/config.toml)
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Additional directories to scan (overrides config watch_directories)
    #[arg(long = "dir", value_name = "PATH")]
    dirs: Vec<PathBuf>,

    /// Run the interactive setup wizard to configure watch directories
    #[arg(long)]
    setup: bool,

    /// Scan once, print results, and exit (no TUI)
    #[arg(long)]
    once: bool,

    /// Output results as JSON — requires --once
    #[arg(long, requires = "once")]
    json: bool,

    /// Print a one-line summary and exit (exit 1 if any repos need attention)
    #[arg(long)]
    summary: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    check_git_installed()?;

    let cli = Cli::parse();

    // First-run detection: config file doesn't exist yet
    let config_path = cli.config.as_ref();
    let is_first_run = config_path
        .map(|p| !p.exists())
        .unwrap_or_else(|| !default_config_path().exists());

    // Run setup wizard if this is the first run or the user explicitly asked for it
    let mut cfg = if cli.setup || is_first_run {
        if is_first_run && !cli.setup {
            println!();
            println!("  Welcome to GitPulse!");
            println!("  No config found — let's pick which directories to scan.");
        }
        let existing = config::load_config(config_path).ok();
        setup::run_setup(existing.as_ref(), cli.config.as_ref())?
    } else {
        config::load_config(config_path)?
    };

    // CLI --dir overrides watch_directories
    if !cli.dirs.is_empty() {
        cfg.watch_directories = cli.dirs.clone();
    }

    if cli.summary {
        let repos = monitor::scan_all(&cfg, &mut StatusCache::new()).await;
        let total = repos.len();
        let dirty = repos.iter().filter(|r| r.needs_attention()).count();
        let unpushed = repos.iter().filter(|r| r.status.unpushed_count > 0).count();
        println!(
            "gitpulse: {} repos | {} dirty | {} unpushed",
            total, dirty, unpushed
        );
        std::process::exit(if dirty > 0 { 1 } else { 0 });
    }

    if cli.once {
        let repos = monitor::scan_all(&cfg, &mut StatusCache::new()).await;
        if cli.json {
            print_json(&repos);
        } else {
            print_table(&repos);
        }
        let any_dirty = repos.iter().any(|r| r.needs_attention());
        std::process::exit(if any_dirty { 1 } else { 0 });
    }

    // In --setup-only mode, stop after writing the config (no TUI)
    if cli.setup {
        return Ok(());
    }

    run_tui(cfg, cli.config).await
}

fn check_git_installed() -> Result<()> {
    match std::process::Command::new("git").arg("--version").output() {
        Ok(o) if o.status.success() => Ok(()),
        _ => Err(anyhow::anyhow!(
            "git is not installed or not in PATH.\nPlease install git and try again."
        )),
    }
}

// ─── TUI ────────────────────────────────────────────────────────────────────

/// Run the TUI, automatically re-launching after setup if the user presses `s`.
async fn run_tui(initial_config: config::Config, config_path: Option<PathBuf>) -> Result<()> {
    // Restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let mut cfg = initial_config;

    loop {
        // ── launch TUI ───────────────────────────────────────────────────────
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let reconfigure = event_loop(&mut terminal, cfg.clone()).await;

        // Always restore terminal before doing anything else
        let _ = disable_raw_mode();
        let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
        let _ = terminal.show_cursor();

        let reconfigure = reconfigure?;

        if !reconfigure {
            break;
        }

        // ── run setup wizard in normal terminal mode, then loop ──────────────
        cfg = setup::run_setup(Some(&cfg), config_path.as_ref())?;
    }

    Ok(())
}

/// Returns `Ok(true)` when the user wants to reconfigure (presses `s`).
async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: config::Config,
) -> Result<bool> {
    let mut app = App::new(config.clone());
    let (scan_tx, mut scan_rx) = tokio::sync::mpsc::channel::<Vec<Repo>>(1);
    let (cache_tx, mut cache_rx) = tokio::sync::mpsc::channel::<StatusCache>(1);
    let (notif_tx, mut notif_rx) = tokio::sync::mpsc::channel::<String>(8);

    // SIGTERM: restore terminal cleanly
    let (term_tx, mut term_rx) = tokio::sync::mpsc::channel::<()>(1);
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        tokio::spawn(async move {
            if let Ok(mut stream) = signal(SignalKind::terminate()) {
                stream.recv().await;
                let _ = term_tx.send(()).await;
            }
        });
    }
    #[cfg(not(unix))]
    drop(term_tx);

    let mut current_cache = StatusCache::new();
    trigger_scan(
        config,
        scan_tx.clone(),
        current_cache.clone(),
        cache_tx.clone(),
    );

    let mut last_refresh = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        // Drain all pending notifications
        while let Ok(msg) = notif_rx.try_recv() {
            app.notify(msg);
        }

        if let Ok(updated) = cache_rx.try_recv() {
            current_cache = updated;
        }

        if let Ok(repos) = scan_rx.try_recv() {
            app.repos = repos;
            app.is_scanning = false;
            app.last_scan = Some(Local::now());
            last_refresh = Instant::now();
        }

        if term_rx.try_recv().is_ok() {
            break;
        }

        app.tick();

        if crossterm::event::poll(Duration::from_millis(100))? {
            match crossterm::event::read()? {
                Event::Key(key) => handle_key(
                    &mut app,
                    key,
                    &scan_tx,
                    &cache_tx,
                    &mut current_cache,
                    &notif_tx,
                ),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if !app.is_scanning {
            let interval = Duration::from_secs(app.config.refresh_interval_secs);
            if last_refresh.elapsed() >= interval {
                trigger_scan(
                    app.config.clone(),
                    scan_tx.clone(),
                    current_cache.clone(),
                    cache_tx.clone(),
                );
                app.is_scanning = true;
                last_refresh = Instant::now();
            }
        }

        if app.should_quit || app.should_reconfigure {
            break;
        }
    }

    Ok(app.should_reconfigure)
}

fn trigger_scan(
    config: config::Config,
    tx: Sender<Vec<Repo>>,
    cache: StatusCache,
    cache_tx: tokio::sync::mpsc::Sender<StatusCache>,
) {
    tokio::spawn(async move {
        let mut cache = cache;
        let repos = monitor::scan_all(&config, &mut cache).await;
        let _ = cache_tx.send(cache).await;
        let _ = tx.send(repos).await;
    });
}

#[allow(clippy::too_many_arguments)]
fn handle_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    scan_tx: &Sender<Vec<Repo>>,
    cache_tx: &tokio::sync::mpsc::Sender<StatusCache>,
    current_cache: &mut StatusCache,
    notif_tx: &tokio::sync::mpsc::Sender<String>,
) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match app.mode {
        AppMode::Normal => match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => app.should_quit = true,
            KeyCode::Char('s') => {
                app.should_reconfigure = true;
                app.should_quit = true;
            }
            KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
            KeyCode::Char('r') => {
                trigger_scan(
                    app.config.clone(),
                    scan_tx.clone(),
                    current_cache.clone(),
                    cache_tx.clone(),
                );
                app.is_scanning = true;
            }
            KeyCode::Char('/') => {
                app.filter_text.clear();
                app.selected = 0;
                app.mode = AppMode::Search;
            }
            KeyCode::Char('?') => app.mode = AppMode::Help,
            KeyCode::Char('g') => {
                app.group_by_dir = !app.group_by_dir;
                app.clamp_selection();
            }
            KeyCode::Enter => {
                if let Some(repo) = app.selected_repo() {
                    let path = repo.path.clone();
                    let editor = app
                        .config
                        .editor
                        .clone()
                        .or_else(|| std::env::var("EDITOR").ok())
                        .unwrap_or_else(|| "code".to_string());
                    let _ = actions::open_in_editor(&path, &editor);
                }
            }
            KeyCode::Char('o') => {
                if let Some(repo) = app.selected_repo() {
                    let path = repo.path.clone();
                    let _ = actions::open_in_file_manager(&path);
                }
            }
            KeyCode::Char('f') => {
                if let Some(repo) = app.selected_repo() {
                    let path = repo.path.clone();
                    let _ = actions::git_fetch(&path);
                }
            }
            KeyCode::Char('p') => {
                if let Some(repo) = app.selected_repo() {
                    let path = repo.path.clone();
                    actions::git_pull(&path, notif_tx.clone());
                    app.notify("Pulling…");
                }
            }
            KeyCode::Char('P') => {
                if let Some(repo) = app.selected_repo() {
                    let path = repo.path.clone();
                    actions::git_push(&path, notif_tx.clone());
                    app.notify("Pushing…");
                }
            }
            KeyCode::Char('c') => {
                app.commit_message.clear();
                app.mode = AppMode::Commit;
            }
            _ => {}
        },
        AppMode::Search => match key.code {
            KeyCode::Esc => {
                app.filter_text.clear();
                app.selected = 0;
                app.mode = AppMode::Normal;
            }
            KeyCode::Enter => app.mode = AppMode::Normal,
            KeyCode::Backspace => {
                app.filter_text.pop();
                app.clamp_selection();
            }
            KeyCode::Char(c) => {
                app.filter_text.push(c);
                app.clamp_selection();
            }
            _ => {}
        },
        AppMode::Help => {
            app.mode = AppMode::Normal;
        }
        AppMode::Commit => match key.code {
            KeyCode::Esc => {
                app.commit_message.clear();
                app.mode = AppMode::Normal;
            }
            KeyCode::Enter => {
                if !app.commit_message.is_empty() {
                    if let Some(repo) = app.selected_repo() {
                        let path = repo.path.clone();
                        let msg = app.commit_message.clone();
                        actions::git_commit(&path, &msg, notif_tx.clone());
                        app.notify(format!("Committing \"{}\"…", msg));
                    }
                }
                app.commit_message.clear();
                app.mode = AppMode::Normal;
            }
            KeyCode::Backspace => {
                app.commit_message.pop();
            }
            KeyCode::Char(c) => {
                app.commit_message.push(c);
            }
            _ => {}
        },
    }
}

// ─── --once output ───────────────────────────────────────────────────────────

fn print_table(repos: &[Repo]) {
    if repos.is_empty() {
        println!("No git repos found. Check your config.");
        return;
    }

    let name_w = repos.iter().map(|r| r.name.len()).max().unwrap_or(4).max(4);
    let branch_w = repos
        .iter()
        .map(|r| r.status.branch.len())
        .max()
        .unwrap_or(6)
        .max(6);

    println!(
        "{:<nw$}  {:<bw$}  {:>11}  {:>5}  STATUS",
        "NAME",
        "BRANCH",
        "UNCOMMITTED",
        "AHEAD",
        nw = name_w,
        bw = branch_w,
    );
    println!("{}", "─".repeat(name_w + branch_w + 34));

    for repo in repos {
        let (indicator, status_label) = match repo.status_color() {
            git::StatusColor::Clean => ("○", "clean"),
            git::StatusColor::Uncommitted => ("●", "uncommitted"),
            git::StatusColor::Unpushed => ("●", "unpushed"),
            git::StatusColor::Dirty => ("●", "dirty"),
            git::StatusColor::NoRemote => ("○", "no remote"),
        };

        let uncommitted = if repo.status.uncommitted_count > 0 {
            repo.status.uncommitted_count.to_string()
        } else {
            "—".to_string()
        };

        let ahead = if repo.status.has_remote {
            if repo.status.unpushed_count > 0 {
                format!("{}↑", repo.status.unpushed_count)
            } else {
                "—".to_string()
            }
        } else {
            "n/a".to_string()
        };

        println!(
            "{} {:<nw$}  {:<bw$}  {:>11}  {:>5}  {}",
            indicator,
            repo.name,
            repo.status.branch,
            uncommitted,
            ahead,
            status_label,
            nw = name_w.saturating_sub(2),
            bw = branch_w,
        );
    }
}

fn print_json(repos: &[Repo]) {
    println!("[");
    let last = repos.len().saturating_sub(1);
    for (i, repo) in repos.iter().enumerate() {
        let comma = if i < last { "," } else { "" };
        println!(
            "  {{\"name\":{:?},\"path\":{:?},\"branch\":{:?},\"uncommitted\":{},\"unpushed\":{},\"behind\":{},\"stash\":{},\"has_remote\":{},\"needs_attention\":{}}}{}",
            repo.name,
            repo.path.to_string_lossy(),
            repo.status.branch,
            repo.status.uncommitted_count,
            repo.status.unpushed_count,
            repo.status.behind_count,
            repo.status.stash_count,
            repo.status.has_remote,
            repo.needs_attention(),
            comma,
        );
    }
    println!("]");
}
