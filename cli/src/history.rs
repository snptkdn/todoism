use chrono::{DateTime, Datelike, Utc, Local, NaiveDate};
use todoism_core::{Task, TaskState};
use todoism_core::time::parse_duration;
use tabled::{Table, Tabled};
use tabled::settings::{Style, Color, Modify};
use tabled::settings::object::{Rows};
use std::collections::HashMap;

// Helper struct for Table Row
#[derive(Tabled)]
struct HistoryRow {
    #[tabled(rename = "Date")]
    date: String,
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Description")]
    desc: String,
    #[tabled(rename = "Est (h)")]
    est: String,
    #[tabled(rename = "Act (h)")]
    act: String,
}

pub fn show_history(tasks: Vec<Task>) {
    let completed_tasks: Vec<&Task> = tasks.iter()
        .filter(|t| matches!(t.state, TaskState::Completed { .. }))
        .collect();

    if completed_tasks.is_empty() {
        println!("No completed tasks found in history.");
        return;
    }

    // Group by ISO Week
    // Key: (Year, WeekNum) -> Value: List of Tasks
    let mut tasks_by_week: HashMap<(i32, u32), Vec<&Task>> = HashMap::new();

    for task in &completed_tasks {
        if let TaskState::Completed { completed_at, .. } = &task.state {
            let local_dt: DateTime<Local> = DateTime::from(*completed_at);
            let iso_week = local_dt.iso_week();
            let key = (iso_week.year(), iso_week.week());
            tasks_by_week.entry(key).or_default().push(task);
        }
    }

    // Sort weeks descending
    let mut sorted_weeks: Vec<_> = tasks_by_week.keys().cloned().collect();
    sorted_weeks.sort_by(|a, b| b.cmp(a));

    for (year, week) in sorted_weeks {
        let tasks_in_week = tasks_by_week.get(&(year, week)).unwrap();

        // Calculate Week Totals
        let (week_est, week_act) = calculate_totals(tasks_in_week);

        // Print Week Header
        println!("\n\x1b[1;36mWeek {}, {}\x1b[0m (Est: {:.1}h, Act: {:.1}h)", week, year, week_est, week_act);

        // Group by Day within Week
        let mut tasks_by_day: HashMap<NaiveDate, Vec<&Task>> = HashMap::new();
        for task in tasks_in_week {
             if let TaskState::Completed { completed_at, .. } = &task.state {
                let local_dt: DateTime<Local> = DateTime::from(*completed_at);
                tasks_by_day.entry(local_dt.date_naive()).or_default().push(task);
            }
        }

        let mut sorted_days: Vec<_> = tasks_by_day.keys().cloned().collect();
        sorted_days.sort_by(|a, b| b.cmp(a));

        // Construct Table Rows
        let mut rows = Vec::new();

        for day in sorted_days {
            let daily_tasks = tasks_by_day.get(&day).unwrap();
            let (day_est, day_act) = calculate_totals(daily_tasks);

            // Add a summary row for the day (Header style in table)
            // Or use the Date column to show the date + summary, and have blank date cells for tasks?
            // "Bat-like" suggestion:
            // 2023-10-25 (Wed) [E: 2h, A: 2h] | ID | Task ...

            let day_header = format!("{} ({})\nE:{:.1}h A:{:.1}h",
                day.format("%Y-%m-%d"),
                day.format("%a"),
                day_est,
                day_act
            );

            // Sort tasks by completion time (if possible) or created
            // We'll just use the iterator order or sort by ID for stability
            let mut daily_tasks_sorted = daily_tasks.clone();
            daily_tasks_sorted.sort_by_key(|t| t.id);

            for (i, task) in daily_tasks_sorted.iter().enumerate() {
                let id_short = task.id.to_string()[..8].to_string();

                // Estimate parsing
                let est_val = parse_est_hours(&task.estimate);
                let est_str = if est_val > 0.0 { format!("{:.1}", est_val) } else { "-".to_string() };

                // Actual parsing
                let act_val = if let TaskState::Completed { actual_duration, .. } = &task.state {
                    *actual_duration as f64 / 3600.0
                } else {
                    0.0
                };
                let act_str = format!("{:.1}", act_val);

                // Date column: Only show on first row of the day group
                let date_col = if i == 0 {
                    day_header.clone()
                } else {
                    String::new()
                };

                rows.push(HistoryRow {
                    date: date_col,
                    id: id_short,
                    desc: task.name.clone(),
                    est: est_str,
                    act: act_str,
                });
            }
        }

        let mut table = Table::new(rows);
        table
            .with(Style::modern())
            .with(Modify::new(Rows::first()).with(Color::FG_CYAN)); // Header color

        // Customize borders to look cool
        // Style::modern() is already quite good (rounded).

        println!("{}", table);
    }
}

fn calculate_totals(tasks: &[&Task]) -> (f64, f64) {
    let mut total_est = 0.0;
    let mut total_act = 0.0;

    for task in tasks {
        total_est += parse_est_hours(&task.estimate);
        if let TaskState::Completed { actual_duration, .. } = &task.state {
            total_act += *actual_duration as f64 / 3600.0;
        }
    }

    (total_est, total_act)
}

fn parse_est_hours(est_opt: &Option<String>) -> f64 {
    if let Some(est) = est_opt {
        if let Ok(duration) = parse_duration(est) {
            return duration.num_minutes() as f64 / 60.0;
        }
    }
    0.0
}
