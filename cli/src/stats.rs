use std::{io, time::Duration};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Bar, BarChart, BarGroup, Block, Borders, BorderType, Paragraph, Gauge, Padding},
};
use todoism_core::{
    repository::{DailyLogRepository, TaskRepository},
    service::{daily_log_service::DailyLogService, dto::WeeklyHistory},
    usecase::history::HistoryUseCase,
};

// --- THEME ---
struct Theme {
    primary: Color,
    muted: Color,
    text: Color,
    act: Color,
    est: Color,
    mtg: Color,
}

const THEME: Theme = Theme {
    primary: Color::Cyan,  // Highlights
    muted: Color::DarkGray,
    text: Color::White,
    act: Color::Green,
    est: Color::Blue,
    mtg: Color::Red,
};

pub struct StatsApp {
    pub histories: Vec<WeeklyHistory>,
    pub current_week_index: usize,
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
                        KeyCode::Left | KeyCode::Char('h') => app.previous_week(),
                        KeyCode::Right | KeyCode::Char('l') => app.next_week(),
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
    let size = frame.area();
    
    // 1. Outer Padding (Window feel)
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header / Tabs
            Constraint::Min(10),   // Main Content (Chart + Sidebar)
            Constraint::Length(1), // Footer / Help
        ])
        .split(size);

    if let Some(history) = app.current_data() {
        // --- Header ---
        let title = format!(" Week {} - {} ", history.week, history.year);
        // Create a "Tab" look for the week selector
        let header_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(THEME.muted));
        
        let header_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20), // "Todoism Stats"
                Constraint::Min(1),     // Spacer
                Constraint::Length(30), // Week Selector
            ])
            .split(main_layout[0]);

        let app_title = Paragraph::new(Span::styled("TODOISM STATS", Style::default().fg(THEME.primary).add_modifier(Modifier::BOLD)))
            .block(Block::default().padding(Padding::new(0,0,1,0)));
        frame.render_widget(app_title, header_layout[0]);

        let nav_text = Line::from(vec![
            Span::styled(" < ", Style::default().fg(if app.current_week_index > 0 { THEME.text } else { THEME.muted })),
            Span::styled(title, Style::default().fg(THEME.text).add_modifier(Modifier::BOLD)),
            Span::styled(" > ", Style::default().fg(if app.current_week_index < app.histories.len() - 1 { THEME.text } else { THEME.muted })),
        ]);
        let nav = Paragraph::new(nav_text).alignment(Alignment::Right).block(Block::default().padding(Padding::new(0,0,1,0)));
        frame.render_widget(nav, header_layout[2]);
        
        frame.render_widget(header_block, main_layout[0]);

        // --- Main Content Split ---
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75), // Chart Area
                Constraint::Length(1),      // Gutter
                Constraint::Percentage(25), // Info Panel
            ])
            .split(main_layout[1]);

        // --- Chart ---
        draw_chart(frame, history, content_chunks[0]);

        // --- Info Panel ---
        draw_info_panel(frame, history, content_chunks[2]);
        
        // --- Footer ---
        let help = Line::from(vec![
            Span::styled("NAV: ", Style::default().fg(THEME.muted)),
            Span::styled("←/→ ", Style::default().fg(THEME.text)),
            Span::raw("  "),
            Span::styled("QUIT: ", Style::default().fg(THEME.muted)),
            Span::styled("q", Style::default().fg(THEME.text)),
        ]);
        let footer = Paragraph::new(help).alignment(Alignment::Center).style(Style::default().fg(THEME.muted));
        frame.render_widget(footer, main_layout[2]);

    } else {
        frame.render_widget(
            Paragraph::new("No data available").alignment(Alignment::Center),
            main_layout[1],
        );
    }
}

fn draw_chart(frame: &mut Frame, history: &WeeklyHistory, area: Rect) {
    let mut bar_data = Vec::new();

    for day in &history.days {
        let act_val = day.stats.total_act_hours / 8.0;
        let est_val = day.stats.total_est_hours / 8.0;
        let mtg_val = day.stats.meeting_hours / 8.0;

        // Act (Green)
        bar_data.push((
            "".to_string(), 
            (act_val * 10.0) as u64, 
            THEME.act
        ));
        
        // Est (Cyan) - Label here
        bar_data.push((
            day.day_of_week.clone(), 
            (est_val * 10.0) as u64, 
            THEME.est
        ));

        // Mtg (Red)
        bar_data.push((
            "".to_string(), 
            (mtg_val * 10.0) as u64, 
            THEME.mtg
        ));
        
        // Spacer
        bar_data.push(("".to_string(), 0, Color::Reset));
    }

    let bar_items: Vec<Bar> = bar_data.iter().map(|(label, value, color)| {
        Bar::default()
            .label(label.as_str())
            .value(*value)
            .style(Style::default().fg(*color))
            .text_value(if *value > 0 { format!("{:.1}", *value as f64 / 10.0) } else { "".to_string() })
    }).collect();

    let chart_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(THEME.muted))
        .title(" Activity Breakdown (Days) ");
        
    let chart = BarChart::default()
        .block(chart_block)
        .bar_width(5)
        .bar_gap(0)
        .data(BarGroup::default().bars(&bar_items))
        .max(15); 

    frame.render_widget(chart, area);
}

fn draw_info_panel(frame: &mut Frame, history: &WeeklyHistory, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Stats
            Constraint::Min(1),     // Legend / Efficiency
        ])
        .split(area);

    // 1. Overview Card
    let stats = &history.stats;
    let total_work = (stats.total_act_hours + stats.meeting_hours) / 8.0;
    
    let info_text = vec![
        Line::from(vec![Span::styled("Overview", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Actual:   ", Style::default().fg(THEME.muted)),
            Span::styled(format!("{:.1}d", stats.total_act_hours / 8.0), Style::default().fg(THEME.act).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Estimate: ", Style::default().fg(THEME.muted)),
            Span::styled(format!("{:.1}d", stats.total_est_hours / 8.0), Style::default().fg(THEME.est).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Meeting:  ", Style::default().fg(THEME.muted)),
            Span::styled(format!("{:.1}d", stats.meeting_hours / 8.0), Style::default().fg(THEME.mtg).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Total:    ", Style::default().fg(THEME.muted)),
            Span::styled(format!("{:.1}d", total_work), Style::default().fg(THEME.text)),
        ]),
    ];

    let info_block = Paragraph::new(info_text)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(THEME.muted)).title(" Summary "));
    frame.render_widget(info_block, chunks[0]);

    // 2. Legend & Gauge
    let est_d = stats.total_est_hours / 8.0;
    let act_d = stats.total_act_hours / 8.0;
    
    // Efficiency: (Est / Act) * 100 ? Or Accuracy: (1 - |Est-Act|/Est)?
    // Let's show "Plan vs Actual" ratio.
    let ratio = if est_d > 0.0 { act_d / est_d } else { 0.0 };
    let percent = ratio * 100.0;
    
    // Gauge
    let label = format!("{:.0}% of Est", percent);
    let gauge = Gauge::default()
        .block(Block::default().title(" Plan Adherence ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(THEME.muted)))
        .gauge_style(Style::default().fg(if ratio > 1.1 { THEME.mtg } else { THEME.act }))
        .ratio(ratio.min(1.0))
        .label(label);
        
    frame.render_widget(gauge, chunks[1]);
}