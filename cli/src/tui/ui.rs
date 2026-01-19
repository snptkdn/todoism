use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Paragraph, Row, Table, Wrap},
    Frame,
};
use todoism_core::{Priority, Status};

use crate::tui::app::App;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Header and Main Content Split
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(1),    // Content
            Constraint::Length(1), // Footer/Help
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
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(main_chunks[1]);

    draw_task_list(f, app, content_chunks[0]);
    draw_detail_view(f, app, content_chunks[1]);

    // Footer
    let footer = Paragraph::new("j/k: Navigate | q: Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(footer, main_chunks[2]);
}

fn draw_task_list(f: &mut Frame, app: &mut App, area: Rect) {
    let rows: Vec<Row> = app.tasks.iter().map(|task| {
        let status_icon = match task.status {
            Status::Completed => "✔",
            Status::Pending => "☐",
            Status::Deleted => "✖",
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

        Row::new(vec![
            Span::styled(status_icon, Style::default()),
            Span::styled(pri_str, priority_style),
            Span::raw(due_str),
            Span::raw(proj_str),
            Span::styled(task.name.clone(), Style::default().add_modifier(Modifier::BOLD)),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),  // Status
            Constraint::Length(3),  // Priority
            Constraint::Length(6),  // Due
            Constraint::Length(10), // Project
            Constraint::Min(10),    // Name
        ]
    )
    .header(Row::new(vec!["St", "Pr", "Due", "Project", "Task"]).style(Style::default().fg(Color::Yellow)))
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
                    Span::raw(format!("{:?}", task.status)),
                ]),
                Line::from(vec![
                    Span::styled("Priority: ", Style::default().fg(Color::Blue)),
                    Span::raw(format!("{:?}", task.priority)),
                ]),
                Line::from(vec![
                    Span::styled("Due: ", Style::default().fg(Color::Blue)),
                    Span::raw(task.due.map(|d| d.to_string()).unwrap_or_else(|| "None".to_string())),
                ]),
                Line::from(vec![
                    Span::styled("Project: ", Style::default().fg(Color::Blue)),
                    Span::raw(task.project.as_deref().unwrap_or("None")),
                ]),
                Line::from(""),
            ];

            if let Some(desc) = &task.description {
                 detail_text.push(Line::from(Span::styled("Description:", Style::default().fg(Color::Blue))));
                 detail_text.push(Line::from(desc.as_str()));
            }

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
