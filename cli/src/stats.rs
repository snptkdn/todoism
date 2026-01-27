use std::{io, time::Duration};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{BarChart, Block, Borders, Paragraph, BorderType},
};
use todoism_core::{
    repository::{DailyLogRepository, TaskRepository},
    service::{daily_log_service::DailyLogService, dto::WeeklyHistory},
    usecase::history::HistoryUseCase,
};
use chrono::{Datelike, Local, NaiveDate};

pub struct StatsApp {
    pub histories: Vec<WeeklyHistory>,
    pub current_week_index: usize, // 0 = oldest, len-1 = newest (current)
    pub today: NaiveDate,
}

impl StatsApp {
    pub fn new(histories: Vec<WeeklyHistory>) -> Self {
        let current_week_index = if histories.is_empty() { 0 } else { histories.len() - 1 };
        Self {
            histories,
            current_week_index,
            today: Local::now().date_naive(),
        }
    }

    pub fn next_week(&mut self) {
        if !self.histories.is_empty() && self.current_week_index < self.histories.len() - 1 {
            self.current_week_index += 1;
        }
    }

    pub fn previous_week(&mut self) {
        if self.current_week_index > 0 {
            self.current_week_index -= 1;
        }
    }

    pub fn current_data(&self) -> Option<&WeeklyHistory> {
        self.histories.get(self.current_week_index)
    }
}

pub fn run<R, L>(task_repo: &R, daily_log_service: &DailyLogService<L>) -> Result<()>
where
    R: TaskRepository,
    L: DailyLogRepository,
{
    // Data setup
    let usecase = HistoryUseCase::new(task_repo, daily_log_service);
    let histories = usecase.get_weekly_history()?;
    
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App setup
    let mut app = StatsApp::new(histories);

    // Main loop
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Left => app.previous_week(),
                        KeyCode::Right => app.next_week(),
                        _ => {}
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(frame: &mut Frame, app: &StatsApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),        // Header
            Constraint::Percentage(50),   // Chart
            Constraint::Percentage(40),   // Heatmap
            Constraint::Length(1),        // Footer
        ])
        .split(frame.area());

    // --- Header ---
    if let Some(history) = app.current_data() {
        let title = format!("Stats: Week {} of {} (Use <Left>/<Right> to navigate)", history.week, history.year);
        let header = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL).title(" Todoism Stats "))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        frame.render_widget(header, chunks[0]);

        // --- Bar Chart ---
        // Prepare data
        let mut bar_data = Vec::new();
        for day in &history.days {
            // Act
            let act_val = day.stats.total_act_hours / 8.0;
            bar_data.push((format!("{} A", day.day_of_week), (act_val * 10.0) as u64, Color::Green));
            // Est
            let est_val = day.stats.total_est_hours / 8.0;
            bar_data.push((format!("{} E", day.day_of_week), (est_val * 10.0) as u64, Color::Blue));
            // Mtg
            let mtg_val = day.stats.meeting_hours / 8.0;
            bar_data.push((format!("{} M", day.day_of_week), (mtg_val * 10.0) as u64, Color::Red));
            
            bar_data.push(("".to_string(), 0, Color::Reset));
        }

        use ratatui::widgets::{Bar, BarGroup};
        let bar_items: Vec<Bar> = bar_data.iter().map(|(label, value, color)| {
            Bar::default()
                .label(label.as_str())
                .value(*value)
                .style(Style::default().fg(*color))
                .text_value(format!("{:.1}", *value as f64 / 10.0)) 
        }).collect();

        let chart = BarChart::default()
            .block(Block::default().title(" Weekly Breakdown (Days) (A=Act, E=Est, M=Mtg) ").borders(Borders::ALL))
            .bar_width(4)
            .bar_gap(1)
            .data(BarGroup::default().bars(&bar_items))
            .max(100); 

        frame.render_widget(chart, chunks[1]);
        
    } else {
        frame.render_widget(
            Paragraph::new("No data available for chart").block(Block::default().borders(Borders::ALL)),
            chunks[1],
        );
    }

    // --- Heatmap (Contribution Graph) ---
    draw_heatmap(frame, app, chunks[2]);

    // --- Footer ---
    let footer_text = "q: Quit | <Left>/<Right>: Navigate Weeks";
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[3]);
}

fn draw_heatmap(f: &mut Frame, app: &StatsApp, area: Rect) {
    let block = Block::default()
        .title(" Weekly Activity Log (Last 12 Weeks) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Data Preparation
    let mut activity_map = std::collections::HashMap::new();
    for wh in &app.histories {
        for dh in &wh.days {
            if let Ok(date) = NaiveDate::parse_from_str(&dh.date, "%Y-%m-%d") {
                let score = dh.stats.total_act_hours + dh.stats.meeting_hours;
                activity_map.insert(date, score);
            }
        }
    }

    let weeks_to_show = 12; 
    let today = app.today;
    
    // Calculate start of current week (Monday)
    let current_weekday_idx = today.weekday().num_days_from_monday(); 
    let start_of_current_week = today - chrono::Duration::days(current_weekday_idx as i64);

    // Build rows (Top = Current Week, Down = Past Weeks)
    // We want headers: "Week Of | Mon Tue Wed Thu Fri Sat Sun | Total"
    
    use ratatui::widgets::{Table, Row};
    
    let header_cells = ["Week Starting", "Mon Tue Wed Thu Fri Sat Sun", "Total"];
    let header = Row::new(header_cells)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .bottom_margin(1);
    
    let mut rows = Vec::new();

    for w in 0..weeks_to_show {
        // Go backwards: 0 = This week, 1 = Last week...
        let week_start = start_of_current_week - chrono::Duration::weeks(w as i64);
        
        let label = week_start.format("%Y-%m-%d").to_string();
        
        // Build the 7-day visualization
        let mut day_cells = Vec::new();
        let mut weekly_total = 0.0;
        
        for d in 0..7 {
            let target_date = week_start + chrono::Duration::days(d);
            let score = activity_map.get(&target_date).cloned().unwrap_or(0.0);
            weekly_total += score;
            
            let color = if target_date > today {
                Color::DarkGray // Future
            } else if score == 0.0 {
                Color::DarkGray // Empty
            } else if score < 2.0 {
                Color::Indexed(22) // Dark Green
            } else if score < 4.0 {
                Color::Indexed(28) // Medium Green
            } else if score < 6.0 {
                Color::Indexed(34) // Green
            } else {
                Color::Indexed(40) // Bright Green
            };
            
            // Wider symbols for better spacing
            let symbol = if target_date > today { " ·  " } else if score == 0.0 { " ·  " } else { " ■  " };
            day_cells.push(Span::styled(symbol, Style::default().fg(color)));
        }
        
        // Join day cells into one Line/Span for the column
        let days_span = Line::from(day_cells);
        
        let total_label = format!("{:.1}h", weekly_total);
        
        rows.push(Row::new(vec![
            ratatui::widgets::Cell::from(label),
            ratatui::widgets::Cell::from(days_span),
            ratatui::widgets::Cell::from(total_label),
        ]));
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(12), // Date label
            Constraint::Length(30), // Days grid (7 * 4 chars)
            Constraint::Min(6),     // Total
        ]
    )
    .header(header)
    .column_spacing(2);

    f.render_widget(table, inner_area);
}
