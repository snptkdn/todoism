use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Paragraph, Row, Table, Wrap},
    Frame,
};
use todoism_core::Priority;
use unicode_width::UnicodeWidthStr;

use crate::tui::app::{App, InputMode};

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Header and Main Content Split
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(1),    // Content
            Constraint::Length(3), // Footer / Input
        ])
        .split(size);

    // Header
    let header = Paragraph::new("TODOISM")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    f.render_widget(header, main_chunks[0]);

    // Split Content into Left (List) and Right (Detail)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(main_chunks[1]);

    draw_task_list(f, app, content_chunks[0]);
    draw_detail_view(f, app, content_chunks[1]);

    // Footer or Input
    match app.input_mode {
        InputMode::Normal => {
            let footer = Paragraph::new("j/k: Navigate | Space: Toggle | d: Delete | a: Add | m: Mod | q: Quit")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            f.render_widget(footer, main_chunks[2]);
        },
        InputMode::Adding => {
             let input = Paragraph::new(app.input.as_str())
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title(" Add Task "))
                .alignment(Alignment::Left);
            f.render_widget(input, main_chunks[2]);
            
            // Cursor
            let cursor_x = app.input.chars().take(app.cursor_position).collect::<String>().width() as u16;
            f.set_cursor_position(
                (
                    main_chunks[2].x + 1 + cursor_x,
                    main_chunks[2].y + 1,
                )
            );
        },
        InputMode::Modifying => {
             let input = Paragraph::new(app.input.as_str())
                .style(Style::default().fg(Color::Green))
                .block(Block::default().borders(Borders::ALL).title(" Modify Task "))
                .alignment(Alignment::Left);
            f.render_widget(input, main_chunks[2]);
            
            // Cursor
            let cursor_x = app.input.chars().take(app.cursor_position).collect::<String>().width() as u16;
            f.set_cursor_position(
                (
                    main_chunks[2].x + 1 + cursor_x,
                    main_chunks[2].y + 1,
                )
            );
        }
    }
}

fn draw_task_list(f: &mut Frame, app: &mut App, area: Rect) {
    let rows: Vec<Row> = app.tasks.iter().map(|task| {
        let (status_icon, status_style) = if task.is_tracking {
             ("▶", Style::default().fg(Color::Green))
        } else {
            match task.status.as_str() {
                "Completed" => ("✔", Style::default()),
                "Pending" => ("☐", Style::default()),
                "Deleted" => ("✖", Style::default()),
                _ => ("?", Style::default()),
            }
        };
        
        let priority_style = match task.priority {
            Priority::High => Style::default().fg(Color::Red),
            Priority::Medium => Style::default().fg(Color::Yellow),
            Priority::Low => Style::default().fg(Color::Green),
        };

        let pri_str = match task.priority {
            Priority::High => "H",
            Priority::Medium => "M",
            Priority::Low => "L",
        };

        let due_str = task.due.map(|d| d.format("%m-%d").to_string()).unwrap_or_else(|| "-".to_string());
        let proj_str = task.project.clone().unwrap_or_else(|| "".to_string());
        let est_str = task.estimate.clone().unwrap_or_else(|| "".to_string());
        let score = task.score;

        Row::new(vec![
            Span::styled(status_icon, status_style),
            Span::styled(format!("{:.1}", score), Style::default().fg(Color::DarkGray)),
            Span::styled(pri_str, priority_style),
            Span::raw(due_str),
            Span::raw(est_str),
            Span::raw(proj_str),
            Span::styled(task.name.clone(), Style::default().add_modifier(Modifier::BOLD)),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),  // Status
            Constraint::Length(5),  // Score
            Constraint::Length(3),  // Priority
            Constraint::Length(6),  // Due
            Constraint::Length(5),  // Est
            Constraint::Length(10), // Project
            Constraint::Min(10),    // Name
        ]
    )
    .header(Row::new(vec!["St", "Score", "Pr", "Due", "Est", "Project", "Task"]).style(Style::default().fg(Color::Yellow)))
    .block(Block::default().title(" Tasks ").borders(Borders::ALL).border_type(BorderType::Rounded))
    .row_highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, &mut app.state);
}

fn draw_detail_view(f: &mut Frame, app: &App, area: Rect) {
    if let Some(selected_index) = app.state.selected() {
        if let Some(task) = app.tasks.get(selected_index) {
            let mut detail_text = vec![
                Line::from(vec![
                    Span::styled("Title: ", Style::default().fg(Color::Blue)),
                    Span::styled(&task.name, Style::default().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("ID: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(task.id.to_string()),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::Blue)),
                    Span::raw(&task.status),
                ]),
                Line::from(vec![
                    Span::styled("Priority: ", Style::default().fg(Color::Blue)),
                    Span::raw(format!("{:?}", task.priority)),
                ]),
                Line::from(vec![
                    Span::styled("Score: ", Style::default().fg(Color::Blue)),
                    Span::raw(format!("{:.2}", task.score)),
                ]),
                Line::from(vec![
                    Span::styled("Due: ", Style::default().fg(Color::Blue)),
                    Span::raw(task.due.map(|d| d.to_string()).unwrap_or_else(|| "None".to_string())),
                ]),
                Line::from(vec![
                    Span::styled("Project: ", Style::default().fg(Color::Blue)),
                    Span::raw(task.project.as_deref().unwrap_or("None")),
                ]),
                Line::from(vec![
                    Span::styled("Estimate: ", Style::default().fg(Color::Blue)),
                    Span::raw(task.estimate.as_deref().unwrap_or("None")),
                ]),
                Line::from(vec![
                    Span::styled("Description: ", Style::default().fg(Color::Blue)),
                    Span::raw(task.description.as_deref().unwrap_or("None")),
                ]),
                Line::from(vec![
                    Span::styled("Time Logged: ", Style::default().fg(Color::Blue)),
                    Span::raw(format!("{}s {}", task.accumulated_time, if task.is_tracking { "(Tracking)" } else { "" })),
                ]),
                Line::from(""),
            ];

            let detail_block = Paragraph::new(detail_text)
                .block(Block::default().title(" Detail ").borders(Borders::ALL).border_type(BorderType::Rounded))
                .wrap(Wrap { trim: true });
            
            f.render_widget(detail_block, area);
        }
    } else {
         let detail_block = Block::default().title(" Detail ").borders(Borders::ALL).border_type(BorderType::Rounded);
         f.render_widget(detail_block, area);
    }
}
