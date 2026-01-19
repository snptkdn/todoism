use std::io;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, BorderType, Paragraph, Row, Table, TableState, Wrap},
    Terminal,
};
use todoism_core::{FileTaskRepository, TaskRepository, Task, Priority, Status};

pub fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

struct App {
    tasks: Vec<Task>,
    state: TableState,
}

impl App {
    fn new() -> App {
        let repo = FileTaskRepository::new(None).expect("Failed to initialize repository");
        let tasks = repo.list().unwrap_or_default();
        let mut state = TableState::default();
        if !tasks.is_empty() {
            state.select(Some(0));
        }
        App { tasks, state }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.tasks.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tasks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
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

            // --- Left Panel: Task Table ---
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
            .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

            f.render_stateful_widget(table, content_chunks[0], &mut app.state);

            // --- Right Panel: Detail View ---
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
                    
                    f.render_widget(detail_block, content_chunks[1]);
                }
            } else {
                 let detail_block = Block::default().title(" Detail ").borders(Borders::ALL).border_type(BorderType::Rounded);
                 f.render_widget(detail_block, content_chunks[1]);
            }

            // Footer
            let footer = Paragraph::new("j/k: Navigate | q: Quit")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            f.render_widget(footer, main_chunks[2]);

        }).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    _ => {}
                }
            }
        }
    }
}
