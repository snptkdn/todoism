use todoism_core::service::dto::{WeeklyHistory, DailyHistory};
use todoism_core::usecase::history::HistoryUseCase;
use todoism_core::repository::{TaskRepository, DailyLogRepository}; 
use tabled::{Table, Tabled};
use tabled::settings::{Style, Color, Modify};
use tabled::settings::object::{Rows};
use anyhow::Result;

// Helper struct for Table Row
#[derive(Tabled)]
struct HistoryRow {
    #[tabled(rename = "Date")]
    date: String,
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Description")]
    desc: String,
    #[tabled(rename = "Est (d)")]
    est: String,
    #[tabled(rename = "Act (d)")]
    act: String,
}

pub fn show_history<R: TaskRepository, L: DailyLogRepository>(history_usecase: &HistoryUseCase<R, L>) -> Result<()> {
    let weekly_history = history_usecase.get_weekly_history()?;

    if weekly_history.is_empty() {
        println!("No completed tasks found in history.");
        return Ok(());
    }

    for week_entry in weekly_history {
        // Print Week Header
        println!("\n\x1b[1;36mWeek {}, {}\x1b[0m (Est: {:.1}d, Act: {:.1}d, Mtg: {:.1}d)", 
                 week_entry.week, 
                 week_entry.year, 
                 week_entry.stats.total_est_hours / 8.0, 
                 week_entry.stats.total_act_hours / 8.0,
                 week_entry.stats.meeting_hours / 8.0);

        // Construct Table Rows
        let mut rows = Vec::new();

        for day_entry in week_entry.days {
            let day_header = format!("{} ({})\nE:{:.1}d A:{:.1}d M:{:.1}d",
                day_entry.date,
                day_entry.day_of_week,
                day_entry.stats.total_est_hours / 8.0,
                day_entry.stats.total_act_hours / 8.0,
                day_entry.stats.meeting_hours / 8.0
            );

            // Sort tasks by ID for stability in display
            let mut daily_tasks_sorted = day_entry.tasks;
            daily_tasks_sorted.sort_by_key(|t| t.id);

            for (i, task_dto) in daily_tasks_sorted.iter().enumerate() {
                let id_short = task_dto.id.to_string()[..8].to_string();

                let est_str = task_dto.estimate.clone().unwrap_or_else(|| "-".to_string());

                let act_str = format!("{:.2}", (task_dto.accumulated_time as f64 / 3600.0) / 8.0);
                
                // Visual distinction for status
                let desc_display = if task_dto.status == "Pending" {
                    format!("{} (In Progress)", task_dto.name) 
                } else {
                    task_dto.name.clone()
                };

                // Date column: Only show on first row of the day group
                let date_col = if i == 0 {
                    day_header.clone()
                } else {
                    String::new()
                };

                rows.push(HistoryRow {
                    date: date_col,
                    id: id_short,
                    desc: desc_display,
                    est: est_str,
                    act: act_str,
                });
            }
        }

        let mut table = Table::new(rows);
        table
            .with(Style::modern())
            .with(Modify::new(Rows::first()).with(Color::FG_CYAN)); // Header color

        println!("{}", table);
    }
    
    Ok(())
}
