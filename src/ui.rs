use ratatui::layout::Alignment;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Table, TableState, Wrap,
};

use crate::app::App;
use crate::status::{parse_ahead_behind, RepoState, NO_CHANGES, NO_LAST_FETCH};
use crate::worker::Action;

const HELP_TEXT: &[&str] = &[
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
    let filtered_indices = app.filtered_indices();
    let filtered_count = filtered_indices.len();
    let total_count = app.repos.len();
    let search_query = app.search_query.clone();
    let status_line = app.status_line.clone();

    // Show empty state if no repos found
    if total_count == 0 && !app.loading {
        render_empty_state(frame, chunks[1]);
    } else if filtered_count == 0 && !search_query.is_empty() {
        render_no_results_state(frame, chunks[1], &search_query);
    } else {
        let table = build_table(&app.repos, &filtered_indices);
        frame.render_stateful_widget(table, chunks[1], &mut app.table_state);
        render_scroll_hints(frame, chunks[1], filtered_count, &app.table_state);
    }

    // Build footer text with appropriate styling
    let (footer_text, footer_style) = if app.search_mode {
        (
            format!("Search: {}_", app.search_query),
            Style::default().fg(Color::Yellow),
        )
    } else if !search_query.is_empty() {
        (
            format!(
                "Filtered: {}/{} repos matching \"{}\" | {}",
                filtered_count, total_count, search_query, status_line
            ),
            Style::default(),
        )
    } else if let Some(action) = &app.confirmation {
        let label = match action {
            Action::Pull => "Pull",
            Action::Push => "Push",
        };
        (
            format!("Confirm {label}? (y/n)"),
            Style::default().fg(Color::Yellow),
        )
    } else if app.loading {
        ("Scanning repositories...".to_string(), Style::default())
    } else {
        // Add timestamp for recent messages (< 5s old)
        let elapsed = app.status_timestamp.elapsed();
        let with_timestamp = if elapsed.as_secs() < 5 {
            format!("{} ({}s ago)", app.status_line, elapsed.as_secs())
        } else {
            app.status_line.clone()
        };

        let color = match app.status_type {
            crate::app::StatusType::Success => Color::Green,
            crate::app::StatusType::Error => Color::Red,
            crate::app::StatusType::Info => Color::Reset,
        };

        (with_timestamp, Style::default().fg(color))
    };

    let footer = Block::default()
        .title("q quit | r refresh | p pull | u push | s sort | / search | ? help")
        .borders(Borders::ALL);
    let footer_paragraph = Paragraph::new(footer_text)
        .block(footer)
        .style(footer_style)
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
    let (ahead_count, behind_count) =
        app.repos
            .iter()
            .fold((0, 0), |(ahead_total, behind_total), repo| {
                if let Some((ahead, behind)) = parse_ahead_behind(&repo.ahead_behind) {
                    (
                        ahead_total + usize::from(ahead > 0),
                        behind_total + usize::from(behind > 0),
                    )
                } else {
                    (ahead_total, behind_total)
                }
            });

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

    let title_paragraph = Paragraph::new(title).wrap(Wrap { trim: true });
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

fn build_table<'a>(repos: &'a [RepoState], indices: &'a [usize]) -> Table<'a> {
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

    let rows = indices
        .iter()
        .filter_map(|idx| repos.get(*idx))
        .map(|repo| {
            let dirty = if repo.dirty { "dirty *" } else { "clean ." };
            let dirty_style = if repo.dirty {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Cyan)
            };

            // Color-code ahead/behind based on status
            let ahead_behind_style = match parse_ahead_behind(&repo.ahead_behind) {
                Some((0, 0)) => Style::default().fg(Color::DarkGray),
                Some((ahead, behind)) if ahead > 0 && behind > 0 => {
                    // Diverged - both ahead and behind
                    Style::default().fg(Color::Red)
                }
                Some((ahead, _)) if ahead > 0 => {
                    // Only ahead
                    Style::default().fg(Color::Green)
                }
                Some((_, behind)) if behind > 0 => {
                    // Only behind
                    Style::default().fg(Color::Yellow)
                }
                _ => Style::default().fg(Color::DarkGray),
            };

            // Show error message in the changes column if present
            let change_cell = if let Some(err) = &repo.error_message {
                Cell::from(format!("⚠ {}", err)).style(Style::default().fg(Color::Red))
            } else {
                Cell::from(colorize_change_summary(&repo.change_summary))
            };

            // Color-code last fetch by staleness
            let fetch_style = get_staleness_style(&repo.last_fetch);

            Row::new(vec![
                Cell::from(repo.name.clone()),
                Cell::from(repo.branch.clone()),
                Cell::from(dirty).style(dirty_style),
                Cell::from(repo.ahead_behind.clone()).style(ahead_behind_style),
                change_cell,
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

fn render_empty_state(frame: &mut Frame, area: Rect) {
    let empty_text = [
        "",
        "",
        "         No Git repositories found",
        "",
        "    Try scanning a different directory:",
        "         git-dash /path/to/projects",
        "",
        "",
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Repositories")
        .style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(empty_text.join("\n"))
        .block(block)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_scroll_hints(frame: &mut Frame, area: Rect, total_rows: usize, state: &TableState) {
    if total_rows == 0 {
        return;
    }

    let inner = Block::default().borders(Borders::ALL).inner(area);
    if inner.height < 2 || inner.width == 0 {
        return;
    }

    let visible_rows = inner.height.saturating_sub(1) as usize;
    if visible_rows == 0 {
        return;
    }

    let offset = state.offset();
    let visible_end = offset.saturating_add(visible_rows);
    let show_top = offset > 0;
    let show_bottom = visible_end < total_rows;

    if !show_top && !show_bottom {
        return;
    }

    let hint_style = Style::default().fg(Color::DarkGray);
    let x = inner.x + inner.width.saturating_sub(1);
    let top_y = inner.y.saturating_add(1);
    let bottom_y = inner.y + inner.height.saturating_sub(1);

    if show_top {
        let rect = Rect {
            x,
            y: top_y,
            width: 1,
            height: 1,
        };
        let hint = Paragraph::new("↑")
            .style(hint_style)
            .alignment(Alignment::Center);
        frame.render_widget(hint, rect);
    }

    if show_bottom {
        let rect = Rect {
            x,
            y: bottom_y,
            width: 1,
            height: 1,
        };
        let hint = Paragraph::new("↓")
            .style(hint_style)
            .alignment(Alignment::Center);
        frame.render_widget(hint, rect);
    }
}

fn render_no_results_state(frame: &mut Frame, area: Rect, query: &str) {
    let message = format!("         No repositories matching \"{}\"", query);
    let no_results_text = [
        "".to_string(),
        "".to_string(),
        message,
        "".to_string(),
        "    Try a different search term or press Esc to clear".to_string(),
        "".to_string(),
        "".to_string(),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Repositories")
        .style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(no_results_text.join("\n"))
        .block(block)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn colorize_change_summary(change_summary: &str) -> Line<'static> {
    if change_summary == NO_CHANGES || change_summary.is_empty() {
        return Line::from(Span::styled("-", Style::default().fg(Color::DarkGray)));
    }

    let mut spans = Vec::new();
    let parts: Vec<&str> = change_summary.split_whitespace().collect();

    for (idx, part) in parts.iter().enumerate() {
        // Each part is like "M:3" or "D:1" or "??:2"
        if let Some(colon_pos) = part.find(':') {
            let change_type = &part[..colon_pos];
            let color = match change_type {
                "M" => Color::Yellow,  // Modified
                "D" => Color::Red,     // Deleted
                "A" => Color::Green,   // Added
                "??" => Color::Cyan,   // Untracked
                "R" => Color::Magenta, // Renamed
                "C" => Color::Blue,    // Copied
                _ => Color::White,     // Unknown
            };

            spans.push(Span::styled(part.to_string(), Style::default().fg(color)));
        } else {
            // Fallback for malformed parts
            spans.push(Span::raw(part.to_string()));
        }

        // Add space separator between parts (but not after the last one)
        if idx < parts.len() - 1 {
            spans.push(Span::raw(" "));
        }
    }

    Line::from(spans)
}

fn get_staleness_style(last_fetch: &str) -> Style {
    // Parse the age from strings like "2d", "5h", "30m", etc.
    if last_fetch == NO_LAST_FETCH {
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

    let help_paragraph = Paragraph::new(HELP_TEXT.join("\n"))
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
