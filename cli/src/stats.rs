use std::{io, time::Duration};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Bar, BarChart, BarGroup, Block, Borders, BorderType, Paragraph, Gauge, Padding, Tabs},
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
    pub current_tab: usize, // 0: Overview, 1: Heatmap
}

impl StatsApp {
    pub fn new(histories: Vec<WeeklyHistory>) -> Self {
        // Start at 0 (Newest week) because histories are sorted Descending (Newest -> Oldest)
        let current_week_index = 0;
        Self {
            histories,
            current_week_index,
            current_tab: 0,
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
    
    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % 2;
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
                        KeyCode::Left | KeyCode::Char('h') => app.next_week(),
                        KeyCode::Right | KeyCode::Char('l') => app.previous_week(),
                        KeyCode::Tab => app.next_tab(),
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
            Constraint::Min(10),   // Main Content
            Constraint::Length(1), // Footer / Help
        ])
        .split(size);

    // --- Header with Tabs ---
    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // "Todoism Stats"
            Constraint::Min(1),     // Tabs
            Constraint::Length(30), // Week Selector (only relevant for Overview)
        ])
        .split(main_layout[0]);

    // Title
    let app_title = Paragraph::new(Span::styled("TODOISM STATS", Style::default().fg(THEME.primary).add_modifier(Modifier::BOLD)))
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(THEME.muted)).padding(Padding::new(0,0,1,0)));
    frame.render_widget(app_title, header_layout[0]);

    // Tabs
    let titles = vec![" Overview ", " Heatmap "];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(THEME.muted)))
        .highlight_style(Style::default().fg(THEME.text).add_modifier(Modifier::BOLD))
        .select(app.current_tab);
    frame.render_widget(tabs, header_layout[1]);

    // Nav (Only show if Overview tab)
    if app.current_tab == 0 {
        if let Some(history) = app.current_data() {
            let title = format!(" Week {} - {} ", history.week, history.year);
            let nav_text = Line::from(vec![
                Span::styled(" < ", Style::default().fg(if app.current_week_index > 0 { THEME.text } else { THEME.muted })),
                Span::styled(title, Style::default().fg(THEME.text).add_modifier(Modifier::BOLD)),
                Span::styled(" > ", Style::default().fg(if app.current_week_index < app.histories.len() - 1 { THEME.text } else { THEME.muted })),
            ]);
            let nav = Paragraph::new(nav_text).alignment(Alignment::Right)
                .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(THEME.muted)).padding(Padding::new(0,0,1,0)));
            frame.render_widget(nav, header_layout[2]);
        }
    } else {
        // Empty block to complete border
        let filler = Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(THEME.muted));
        frame.render_widget(filler, header_layout[2]);
    }

    // --- Main Content ---
    match app.current_tab {
        0 => {
             if let Some(history) = app.current_data() {
                let content_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(75), // Chart Area
                        Constraint::Length(1),      // Gutter
                        Constraint::Percentage(25), // Info Panel
                    ])
                    .split(main_layout[1]);

                draw_chart(frame, history, content_chunks[0]);
                draw_info_panel(frame, history, content_chunks[2]);
            } else {
                frame.render_widget(Paragraph::new("No data"), main_layout[1]);
            }
        },
        1 => {
            draw_heatmap(frame, &app.histories, main_layout[1]);
        },
        _ => {}
    }

    // --- Footer ---
    let help_text = if app.current_tab == 0 {
        vec![
            Span::styled("NAV: ", Style::default().fg(THEME.muted)),
            Span::styled("←/→ ", Style::default().fg(THEME.text)),
            Span::raw("  "),
            Span::styled("TAB: ", Style::default().fg(THEME.muted)),
            Span::styled("Switch View ", Style::default().fg(THEME.text)),
            Span::raw("  "),
            Span::styled("QUIT: ", Style::default().fg(THEME.muted)),
            Span::styled("q", Style::default().fg(THEME.text)),
        ]
    } else {
        vec![
            Span::styled("TAB: ", Style::default().fg(THEME.muted)),
            Span::styled("Switch View ", Style::default().fg(THEME.text)),
            Span::raw("  "),
            Span::styled("QUIT: ", Style::default().fg(THEME.muted)),
            Span::styled("q", Style::default().fg(THEME.text)),
        ]
    };
    
    let footer = Paragraph::new(Line::from(help_text)).alignment(Alignment::Center).style(Style::default().fg(THEME.muted));
    frame.render_widget(footer, main_layout[2]);
}

fn draw_heatmap(frame: &mut Frame, histories: &Vec<WeeklyHistory>, area: Rect) {
    // 1. Group by Year
    let mut years_map: std::collections::HashMap<i32, Vec<&WeeklyHistory>> = std::collections::HashMap::new();
    for h in histories {
        years_map.entry(h.year).or_default().push(h);
    }
    
    // 2. Sort Years Descending
    let mut sorted_years: Vec<i32> = years_map.keys().cloned().collect();
    sorted_years.sort_by(|a, b| b.cmp(a)); // 2026, 2025...

    if sorted_years.is_empty() { return; }

    // 3. Calculate Layout
    // Each year needs approx 8 lines (Border + MonthHeader + 4 GridLines + Spacer?)
    // Actually grid is 7 lines (Mon-Sun).
    // Layout:
    //  Border Top
    //  Month Header (1)
    //  Spacer (1) - matching left col
    //  Grid (7)
    //  Border Bottom
    // Total internal height = 1 + 1 + 7 = 9 lines.
    // Plus borders = 11 lines per year block.
    
    let year_height = 11;
    let total_height = area.height;
    
    // Check how many years fit
    let count = (total_height as usize / year_height).max(1);
    let visible_years = sorted_years.iter().take(count);
    
    let constraints: Vec<Constraint> = visible_years.clone().map(|_| Constraint::Length(year_height as u16)).collect();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
        
    for (i, &year) in visible_years.enumerate() {
        if let Some(year_data) = years_map.get(&year) {
             draw_year_heatmap(frame, year, year_data, chunks[i]);
        }
    }
}

fn draw_year_heatmap(frame: &mut Frame, year: i32, histories: &Vec<&WeeklyHistory>, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(THEME.muted))
        .title(format!(" {} ", year)); // Year Title
    
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let grid_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(4), // Day labels
            Constraint::Min(1),    // Heatmap
        ])
        .split(inner_area);
        
    // Left column layout
    let day_labels_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
             Constraint::Length(1), // Spacer for Month Header
             Constraint::Min(1),    // Labels
        ])
        .split(grid_layout[0]);
        
    // Right column layout
    let labels_vs_grid = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Months
            Constraint::Min(1),    // Grid
        ])
        .split(grid_layout[1]);

    // Render Logic (Similar to previous, but scoped to year_data)
    // Note: year_data is already filtered by year.
    // However, it might be sparse or unsorted?
    // histories in `StatsApp` are sorted Descending (Newest first).
    // So `year_data` (Vec<&WeeklyHistory>) is also Newest->Oldest.
    
    // We want Past->Present (Left->Right).
    // So we need to reverse the order for display.
    // Also we need to make sure we fill the whole year? 
    // Or just available data? 
    // If we want a full calendar grid, we might need 52/53 weeks.
    // Let's stick to "Available Data" for now to avoid complexity of empty weeks generation.
    // But for a "Calendar" look, fixed width is better.
    
    // Let's reverse to get Jan -> Dec
    let view_slice: Vec<&&WeeklyHistory> = histories.iter().rev().collect();
    
    // --- Draw Month Labels ---
    let mut month_spans = Vec::new();
    let mut last_month = String::new();
    
    for history in &view_slice {
         let month_str = if let Some(day) = history.days.first() {
             if day.date.len() >= 7 { &day.date[5..7] } else { "" }
         } else { "" };
         
         let label_name = match month_str {
             "01" => "Jan", "02" => "Feb", "03" => "Mar", "04" => "Apr",
             "05" => "May", "06" => "Jun", "07" => "Jul", "08" => "Aug",
             "09" => "Sep", "10" => "Oct", "11" => "Nov", "12" => "Dec",
             _ => "",
         };

         if label_name != last_month && !label_name.is_empty() {
             last_month = label_name.to_string();
             month_spans.push(Span::styled(format!("{:<2}", &label_name[0..2]), Style::default().fg(THEME.text)));
         } else {
             month_spans.push(Span::raw("  "));
         }
    }
    
    frame.render_widget(Paragraph::new(Line::from(month_spans)), labels_vs_grid[0]);

    // --- Draw Day Labels ---
    let day_rows = vec![
        Line::from(Span::styled("Mon ", Style::default().fg(THEME.muted))),
        Line::from(""),
        Line::from(Span::styled("Wed ", Style::default().fg(THEME.muted))),
        Line::from(""),
        Line::from(Span::styled("Fri ", Style::default().fg(THEME.muted))),
        Line::from(""),
        Line::from(""),
    ];
    frame.render_widget(Paragraph::new(day_rows), day_labels_layout[1]);

    // --- Draw Grid ---
    let mut grid_lines: Vec<Line> = Vec::new();
    let mut grid_data = Vec::new();
    
    for history in &view_slice {
        let mut week_hours = vec![0.0; 7];
        for day in &history.days {
             let idx = match day.day_of_week.as_str() {
                "Mon" => 0, "Tue" => 1, "Wed" => 2, "Thu" => 3, "Fri" => 4, "Sat" => 5, "Sun" => 6,
                _ => 0,
            };
            if idx < 7 {
                week_hours[idx] = day.stats.total_act_hours;
            }
        }
        grid_data.push(week_hours);
    }

    for row_idx in 0..7 {
        let mut spans = Vec::new();
        for col_idx in 0..grid_data.len() {
             let hours = grid_data[col_idx][row_idx];
             let color = get_heat_color(hours);
             spans.push(Span::styled("  ", Style::default().bg(color)));
        }
        grid_lines.push(Line::from(spans));
    }
    
    frame.render_widget(Paragraph::new(grid_lines), labels_vs_grid[1]);
}

fn get_heat_color(hours: f64) -> Color {
    // RGB Gradient: Dark Gray -> Cyan/Teal
    // Base (Empty): 30, 30, 30
    // L1: 20, 60, 60
    // L2: 30, 100, 100
    // L3: 40, 160, 160
    // L4: 50, 220, 220
    
    if hours <= 0.1 { Color::Rgb(30, 30, 30) }
    else if hours < 2.0 { Color::Rgb(22, 57, 57) }
    else if hours < 4.0 { Color::Rgb(30, 93, 93) }
    else if hours < 6.0 { Color::Rgb(39, 137, 137) }
    else if hours < 8.0 { Color::Rgb(51, 182, 182) }
    else { Color::Rgb(78, 222, 222) } // Max intensity
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