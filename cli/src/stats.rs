use std::{io, time::Duration};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{BarChart, Block, Borders, Paragraph},
};
use todoism_core::{
    repository::{DailyLogRepository, TaskRepository},
    service::{daily_log_service::DailyLogService, dto::WeeklyHistory},
    usecase::history::HistoryUseCase,
};

pub struct StatsApp {
    pub histories: Vec<WeeklyHistory>,
    pub current_week_index: usize, // 0 = oldest, len-1 = newest (current)
}

impl StatsApp {
    pub fn new(histories: Vec<WeeklyHistory>) -> Self {
        let current_week_index = if histories.is_empty() { 0 } else { histories.len() - 1 };
        Self {
            histories,
            current_week_index,
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
    
    if histories.is_empty() {
        println!("No history data available.");
        return Ok(());
    }

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
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Chart
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    if let Some(history) = app.current_data() {
        // Header
        let title = format!("Stats: Week {} of {} (Use <Left>/<Right> to navigate)", history.week, history.year);
        let header = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL).title(" Todoism Stats "))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        frame.render_widget(header, chunks[0]);

        // Content (Chart)
        // Group bars by day. Ratatui BarChart doesn't support grouped bars natively well in simple mode,
        // but we can show Act vs Est vs Mtg.
        // Let's create a combined view or separated. 
        // For TUI "wow", maybe 3 separate charts or one mixed?
        // Let's try one bar chart that shows Total Work (Act + Mtg) vs Estimate?
        // Or maybe just Act, Est, Mtg side by side is hard.
        // Let's do a Stacked Bar Chart manually or just discrete bars for "Act" and "Mtg".
        // 
        // Better: 3 datasets? Ratatui 0.28+ has BarChart::data(Grouped).
        // Let's stick to simple BarChart for now, maybe just showing "Act" (Green) and "Mtg" (Red) stacked?
        // Actually, let's just show "Actual Work" (Act+Mtg) vs "Estimate" bars side by side?
        // No, user wants Act, Est, Mtg relation.
        // 
        // Let's map each day to 3 bars: e.g. "Mon A", "Mon E", "Mon M". 
        // Labels: "Mo-A", "Mo-E", "Mo-M".
        
        // Prepare data
        // We will construct a Vec<String> to hold labels alive.
        
        // Prepare data
        // We need to keep labels alive.
        let mut bar_data = Vec::new();

        for day in &history.days {
            // Act
            bar_data.push((format!("{} A", day.day_of_week), (day.stats.total_act_hours * 10.0) as u64, Color::Green));
            // Est
            bar_data.push((format!("{} E", day.day_of_week), (day.stats.total_est_hours * 10.0) as u64, Color::Blue));
            // Mtg
            bar_data.push((format!("{} M", day.day_of_week), (day.stats.meeting_hours * 10.0) as u64, Color::Red));
            
            // Spacer?
            bar_data.push(("".to_string(), 0, Color::Reset));
        }

        // Convert to what BarChart expects
        // BarChart::default().data(&[("Label", value), ...])
        // But we want colors. 
        // Ratatui BarChart allows setting styles for bars. But to have different colors per bar?
        // The standard BarChart applies one style.
        // We might need `Bar::default().value(...).style(...)` if using the new API.
        // Ratatui 0.30 should support `BarGroup` or `Bar`.
        
        // Use explicit types from widgets module
        use ratatui::widgets::{Bar, BarGroup};
        
        let bar_items: Vec<Bar> = bar_data.iter().map(|(label, value, color)| {
            Bar::default()
                .label(label.as_str())
                .value(*value)
                .style(Style::default().fg(*color))
                .text_value(format!("{:.1}", *value as f64 / 10.0)) // Show real float value
        }).collect();

        let chart = BarChart::default()
            .block(Block::default().title("Daily Breakdown (A=Act, E=Est, M=Mtg)").borders(Borders::ALL))
            .bar_width(5)
            .bar_gap(1)
            .data(BarGroup::default().bars(&bar_items))
            .max(300); // 30.0 hours max?

        frame.render_widget(chart, chunks[1]);
        
        // Footer (Summary)
        let stats = &history.stats;
        let summary = format!(
            "Total: Act {:.1}h | Est {:.1}h | Mtg {:.1}h | Total Work {:.1}h",
            stats.total_act_hours,
            stats.total_est_hours,
            stats.meeting_hours,
            stats.total_act_hours + stats.meeting_hours
        );
        let footer = Paragraph::new(summary)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(footer, chunks[2]);

    } else {
        frame.render_widget(
            Paragraph::new("No data").block(Block::default().borders(Borders::ALL)),
            chunks[0],
        );
    }
}
