use crate::repository::TaskRepository;
use crate::service::daily_log_service::DailyLogService;
use crate::repository::DailyLogRepository;
use crate::service::dto::{TaskDto, WeeklyHistory, DailyHistory, HistoryStats};
use crate::model::task::TaskState;
use crate::service::task_service::parse_est_hours; // Need to expose or duplicate this helper
use chrono::{DateTime, Local, Datelike};
use anyhow::Result;
use std::collections::HashMap;

pub struct HistoryUseCase<'a, R: TaskRepository, L: DailyLogRepository> {
    task_repo: &'a R,
    daily_log_service: &'a DailyLogService<L>,
}

impl<'a, R: TaskRepository, L: DailyLogRepository> HistoryUseCase<'a, R, L> {
    pub fn new(task_repo: &'a R, daily_log_service: &'a DailyLogService<L>) -> Self {
        Self {
            task_repo,
            daily_log_service,
        }
    }

    pub fn get_weekly_history(&self) -> Result<Vec<WeeklyHistory>> {
        let mut weekly_data: HashMap<(i32, u32), HashMap<chrono::NaiveDate, (Vec<TaskDto>, f64, f64, f64)>> = HashMap::new();
        // Map: (Year, Week) -> Date -> (Tasks, EstHours, ActHours, MtgHours)

        let tasks = self.task_repo.list()?;
        // Filter tasks: Completed tasks OR Pending tasks with time logs
        let eligible_tasks: Vec<_> = tasks.iter()
            .filter(|t| match &t.state {
                TaskState::Completed { .. } => true,
                TaskState::Pending { time_logs } => !time_logs.is_empty(),
                _ => false,
            })
            .collect();

        // Pass 1: Place tasks in listing slots and distribute actual hours
        for task in &eligible_tasks {
            let task_dto = TaskDto::from_entity((*task).clone(), 0.0);
            
            match &task.state {
                TaskState::Completed { completed_at, actual, time_logs } => {
                     let local_dt: DateTime<Local> = DateTime::from(*completed_at);
                     let date = local_dt.date_naive();
                     let iso = local_dt.iso_week();
                     let week_key = (iso.year(), iso.week());
                     
                     let entry = weekly_data.entry(week_key).or_default().entry(date).or_default();
                     entry.0.push(task_dto);
                     
                     let est = parse_est_hours(&task.estimate);
                     entry.1 += est;
                     
                     // Distribute logs
                     if time_logs.is_empty() {
                         if let Some(act_str) = actual {
                             if let Ok(days) = act_str.parse::<f64>() {
                                 entry.2 += days * 8.0;
                             }
                         }
                     } else {
                         distribute_logs(time_logs, &mut weekly_data);
                     }
                },
                TaskState::Pending { time_logs } => {
                    // For pending tasks, we don't have a single "completion date".
                    // We should list them on the days they were worked on? 
                    // Or usually, checking history implies "what did I do today".
                    // If I worked on it today, it should appear in today's history list.
                    // But if I worked on it yesterday, it should appear in yesterday's list.
                    // This means a single pending task might appear multiple times in history lists if worked on multiple days.
                    
                    // Logic: Iterate logs. For each Unique Day involved in logs, add this task to that day's list.
                    // Warning: This duplicates the task in the list view, but that's arguably correct for a "timesheet" view.
                    
                    // Distribute logs first to get hours right
                    distribute_logs(time_logs, &mut weekly_data);
                    
                    // Now ensure task is listed on days it has activity
                    let mut days_active = std::collections::HashSet::new();
                    for log in time_logs {
                         let log_local: DateTime<Local> = DateTime::from(log.start);
                         days_active.insert(log_local.date_naive());
                         if let Some(end) = log.end {
                              let end_local: DateTime<Local> = DateTime::from(end);
                              days_active.insert(end_local.date_naive());
                         }
                    }
                    
                    for date in days_active {
                        let iso = date.iso_week(); // Approximate, using date's iso week
                         let week_key = (iso.year(), iso.week());
                         let entry = weekly_data.entry(week_key).or_default().entry(date).or_default();
                         
                         // Check if already added to avoid dupes if multiple logs on same day?
                         // The entry.0 is a Vec<TaskDto>. 
                         // We just reconstructed it. 
                         // To be safe, maybe check ID? But simplified: just push.
                         // Optimization: verify uniqueness if needed.
                         
                         // Note: We don't add Estimate hours for Pending tasks to the "Verified/Completed Est" sum?
                         // Ideally, "Total Est Hours" in history usually means "Velocity" (completed work).
                         // If we add Pending work, it inflates velocity without completion.
                         // Let's NOT add estimate for Pending tasks.
                         
                         entry.0.push(task_dto.clone());
                    }
                },
                _ => {}
            }
        }
        
        // Pass 2: Generate final history structure, adding meeting hours
        let mut history = Vec::new();
        let mut sorted_weeks: Vec<_> = weekly_data.keys().cloned().collect();
        sorted_weeks.sort_by(|a, b| b.cmp(a));

        for (year, week) in sorted_weeks {
            let days_map = weekly_data.get(&(year, week)).unwrap();
            let mut sorted_days: Vec<_> = days_map.keys().cloned().collect();
            sorted_days.sort_by(|a, b| b.cmp(a));
            
            let mut daily_histories = Vec::new();
            let mut week_est = 0.0;
            let mut week_act = 0.0;
            let mut week_mtg = 0.0;
            
            for day in sorted_days {
                let (day_tasks, est, act, _) = days_map.get(&day).cloned().unwrap(); 
                
                // Get meeting hours for this day
                let mtg = self.daily_log_service.get_log(day).ok().flatten().map(|l| l.total_hours()).unwrap_or(0.0);
                
                week_est += est;
                week_act += act;
                week_mtg += mtg;
                
                daily_histories.push(DailyHistory {
                    date: day.format("%Y-%m-%d").to_string(),
                    day_of_week: day.format("%a").to_string(),
                    tasks: day_tasks,
                    stats: HistoryStats {
                        total_est_hours: est,
                        total_act_hours: act,
                        meeting_hours: mtg,
                    }
                });
            }
            
            history.push(WeeklyHistory {
                year,
                week,
                days: daily_histories,
                stats: HistoryStats {
                    total_est_hours: week_est,
                    total_act_hours: week_act,
                    meeting_hours: week_mtg,
                }
            });
        }
        
        Ok(history)
    }
}

// Helper to distribute logs into weekly_data
fn distribute_logs(
    logs: &Vec<crate::model::task::TimeLog>, 
    weekly_data: &mut HashMap<(i32, u32), HashMap<chrono::NaiveDate, (Vec<TaskDto>, f64, f64, f64)>>
) {
    for log in logs {
        if let Some(end) = log.end {
            let log_local: DateTime<Local> = DateTime::from(log.start);
            let log_date = log_local.date_naive();
            let log_iso = log_local.iso_week();
            let log_week_key = (log_iso.year(), log_iso.week());
            
            let start_ts = log.start.timestamp();
            let end_ts = end.timestamp();
            let dur_sec = end_ts - start_ts;
            if dur_sec > 0 {
                let hrs = dur_sec as f64 / 3600.0;
                let log_entry = weekly_data.entry(log_week_key).or_default().entry(log_date).or_default();
                log_entry.2 += hrs;
            }
        }
    }
}
