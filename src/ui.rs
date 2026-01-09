use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Clear, Gauge, Row, Table, Wrap};

use crate::app::App;
use crate::status::RepoState;
use crate::worker::Action;

pub fn render_ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_header(frame, chunks[0], app);

    // Get filtered repos and their count before borrowing table_state mutably
    let filtered_repos = app.filtered_repos();
    let filtered_count = filtered_repos.len();
    let total_count = app.repos.len();
    let search_query = app.search_query.clone();
    let status_line = app.status_line.clone();

    let table = build_table(&filtered_repos);
    frame.render_stateful_widget(table, chunks[1], &mut app.table_state);

    let footer_text = if app.search_mode {
        format!("Search: {}_", app.search_query)
    } else if !search_query.is_empty() {
        format!(
            "Filtered: {}/{} repos matching \"{}\" | {}",
            filtered_count, total_count, search_query, status_line
        )
    } else if let Some(action) = &app.confirmation {
        let label = match action {
            Action::Pull => "Pull",
            Action::Push => "Push",
        };
        format!("Confirm {label}? (y/n)")
    } else if app.loading {
        "Scanning repositories...".to_string()
    } else {
        app.status_line.clone()
    };

    let footer = Block::default()
        .title("q quit | r refresh | p pull | u push | s sort | / search | ? help")
        .borders(Borders::ALL);
    let footer_paragraph = ratatui::widgets::Paragraph::new(footer_text)
        .block(footer)
        .wrap(Wrap { trim: true });
    frame.render_widget(footer_paragraph, chunks[2]);

    // Render help overlay on top if visible
    if app.help_visible {
        render_help_overlay(frame);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(32)])
        .split(inner);

    // Calculate repository statistics
    let total_repos = app.repos.len();
    let dirty_count = app.repos.iter().filter(|r| r.dirty).count();
    let ahead_count = app
        .repos
        .iter()
        .filter(|r| r.ahead_behind.contains('↑'))
        .count();
    let behind_count = app
        .repos
        .iter()
        .filter(|r| r.ahead_behind.contains('↓'))
        .count();

    let title = if !app.loading && total_repos > 0 {
        format!(
            "git-dash — {} │ {} repos │ {} dirty │ {} ahead │ {} behind",
            app.root.display(),
            total_repos,
            dirty_count,
            ahead_count,
            behind_count
        )
    } else {
        format!("git-dash — {}", app.root.display())
    };

    let title_paragraph = ratatui::widgets::Paragraph::new(title).wrap(Wrap { trim: true });
    frame.render_widget(title_paragraph, header_chunks[0]);

    if app.loading {
        let ratio = app.scan_progress.clamp(0.0, 1.0);
        let percent = (ratio * 100.0).round() as u16;
        let gauge = Gauge::default()
            .ratio(ratio)
            .label(format!("Scanning {percent}%"))
            .gauge_style(Style::default().fg(Color::Blue).bg(Color::Black));
        frame.render_widget(gauge, header_chunks[1]);
    }
}

fn build_table(repos: &[RepoState]) -> Table<'_> {
    let header = Row::new(vec![
        Cell::from("Repository"),
        Cell::from("Branch"),
        Cell::from("Dirty"),
        Cell::from("Ahead/Behind"),
        Cell::from("Changes"),
        Cell::from("Remote"),
        Cell::from("Last Fetch"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let rows = repos.iter().map(|repo| {
        let dirty = if repo.dirty { "dirty *" } else { "clean ." };
        let dirty_style = if repo.dirty {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        };

        // Color-code ahead/behind based on status
        let ahead_behind_style = if repo.ahead_behind == "-" {
            Style::default().fg(Color::DarkGray)
        } else if repo.ahead_behind.contains('↑') && repo.ahead_behind.contains('↓') {
            // Diverged - both ahead and behind
            Style::default().fg(Color::Red)
        } else if repo.ahead_behind.contains('↑') {
            // Only ahead
            Style::default().fg(Color::Green)
        } else if repo.ahead_behind.contains('↓') {
            // Only behind
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        // Show error message in the changes column if present
        let (change_display, has_error) = if let Some(err) = &repo.error_message {
            (format!("⚠ {}", err), true)
        } else {
            (repo.change_summary.clone(), false)
        };

        // Style the change column - red if error, default otherwise
        let change_style = if has_error {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        };

        // Color-code last fetch by staleness
        let fetch_style = get_staleness_style(&repo.last_fetch);

        Row::new(vec![
            Cell::from(repo.name.clone()),
            Cell::from(repo.branch.clone()),
            Cell::from(dirty).style(dirty_style),
            Cell::from(repo.ahead_behind.clone()).style(ahead_behind_style),
            Cell::from(change_display).style(change_style),
            Cell::from(repo.remote_url.clone()),
            Cell::from(repo.last_fetch.clone()).style(fetch_style),
        ])
    });

    Table::new(
        rows,
        [
            Constraint::Percentage(18),
            Constraint::Percentage(10),
            Constraint::Percentage(8),
            Constraint::Percentage(12),
            Constraint::Percentage(22),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Repositories"))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
}

fn get_staleness_style(last_fetch: &str) -> Style {
    // Parse the age from strings like "2d", "5h", "30m", etc.
    if last_fetch == "-" {
        return Style::default().fg(Color::DarkGray);
    }

    // Extract the numeric value and unit
    let trimmed = last_fetch.trim();
    if let Some(d_pos) = trimmed.find('d') {
        // Days
        if let Ok(days) = trimmed[..d_pos].parse::<u64>() {
            return if days == 0 {
                Style::default().fg(Color::Green)
            } else if days < 7 {
                Style::default()
            } else if days < 30 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Red)
            };
        }
    } else if trimmed.ends_with('h') || trimmed.ends_with('m') || trimmed.ends_with('s') {
        // Hours, minutes, or seconds - all less than a day
        return Style::default().fg(Color::Green);
    }

    Style::default()
}

fn render_help_overlay(frame: &mut Frame) {
    let area = frame.area();

    // Create centered popup area
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 20.min(area.height.saturating_sub(4));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Help text content
    let help_text = vec![
        "NAVIGATION",
        "  j / ↓          Move selection down",
        "  k / ↑          Move selection up",
        "  PgDn           Page down",
        "  PgUp           Page up",
        "  g / Home       Jump to first repository",
        "  G / End        Jump to last repository",
        "",
        "ACTIONS",
        "  p              Pull (with confirmation)",
        "  u              Push (with confirmation)",
        "  r              Refresh repository status",
        "",
        "VIEW",
        "  s              Cycle sort order (Name → Status → Ahead/Behind → Last Fetch)",
        "  /              Search/filter repositories by name",
        "  Esc            Clear search filter",
        "  ?              Toggle this help screen",
        "",
        "OTHER",
        "  q / Ctrl+C     Quit git-dash",
        "  y              Confirm action",
        "  n / Esc        Cancel action",
    ];

    let help_paragraph = ratatui::widgets::Paragraph::new(help_text.join("\n"))
        .block(
            Block::default()
                .title(" Help (press any key to close) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Black))
        .wrap(Wrap { trim: false });

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);
    frame.render_widget(help_paragraph, popup_area);
}
