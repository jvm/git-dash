use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Gauge, Row, Table, Wrap};

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
        .split(frame.size());

    render_header(frame, chunks[0], app);

    let table = build_table(&app.repos);
    frame.render_stateful_widget(table, chunks[1], &mut app.table_state);

    let footer_text = if let Some(action) = &app.confirmation {
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
        .title("q quit | r refresh | p pull | u push | j/k ↓↑ | PgUp/PgDn | Esc cancel")
        .borders(Borders::ALL);
    let footer_paragraph = ratatui::widgets::Paragraph::new(footer_text)
        .block(footer)
        .wrap(Wrap { trim: true });
    frame.render_widget(footer_paragraph, chunks[2]);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(32)])
        .split(inner);

    let title = ratatui::widgets::Paragraph::new(format!("git-dash — {}", app.root.display()))
        .wrap(Wrap { trim: true });
    frame.render_widget(title, header_chunks[0]);

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

        // Show error message in the changes column if present
        let change_display = if let Some(err) = &repo.error_message {
            format!("{} ({})", repo.change_summary, err)
        } else {
            repo.change_summary.clone()
        };

        Row::new(vec![
            Cell::from(repo.name.clone()),
            Cell::from(repo.branch.clone()),
            Cell::from(dirty).style(dirty_style),
            Cell::from(repo.ahead_behind.clone()),
            Cell::from(change_display),
            Cell::from(repo.remote_url.clone()),
            Cell::from(repo.last_fetch.clone()),
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
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
}
