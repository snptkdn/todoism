use crate::repository::{TaskRepository, DailyLogRepository, FileStatsRepository};
use crate::service::daily_log_service::DailyLogService;
use crate::service::dto::{TaskDto, WeeklyHistory, DailyHistory, HistoryStats};
use crate::model::task::TaskState;
use crate::service::task_service::parse_est_hours;
use chrono::{DateTime, Local, Datelike, NaiveDate};
use anyhow::Result;
use std::collections::HashMap;

pub struct HistoryUseCase<'a, R: TaskRepository, L: DailyLogRepository> {
    task_repo: &'a R,
    daily_log_service: &'a DailyLogService<L>,
    stats_repo: &'a FileStatsRepository,
}

impl<'a, R: TaskRepository, L: DailyLogRepository> HistoryUseCase<'a, R, L> {
    pub fn new(task_repo: &'a R, daily_log_service: &'a DailyLogService<L>, stats_repo: &'a FileStatsRepository) -> Self {
        Self {
            task_repo,
            daily_log_service,
            stats_repo,
        }
    }

    pub fn get_weekly_history(&self) -> Result<Vec<WeeklyHistory>> {
        let mut weekly_data: HashMap<(i32, u32), HashMap<chrono::NaiveDate, (Vec<TaskDto>, f64, f64, f64)>> = HashMap::new();
        // Map: (Year, Week) -> Date -> (Tasks, EstHours, ActHours, MtgHours)

        // 1. Load from Stats Repository (Archived Data)
        let stats_list = self.stats_repo.list_stats()?;
        for monthly_stats in stats_list {
            for (date_str, daily_stats) in monthly_stats.days {
                if let Ok(date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    // Determine Week
                    // Note: We use ISO week to match other logic
                    let iso = date.iso_week();
                    let week_key = (iso.year(), iso.week());
                    
                    let entry = weekly_data.entry(week_key).or_default().entry(date).or_default();
                    
                    // Add stats
                    entry.1 += daily_stats.est;
                    entry.2 += daily_stats.act;
                    entry.3 += daily_stats.mtg;
                }
            }
        }

        // 2. Load from Task Repository (Current Data)
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
            sorted_days.sort_by(|a, b| a.cmp(b));
            
            let mut daily_histories = Vec::new();
            let mut week_est = 0.0;
            let mut week_act = 0.0;
            let mut week_mtg = 0.0;
            
            for day in sorted_days {
                let (day_tasks, est, act, _) = days_map.get(&day).cloned().unwrap(); 
                
                // Get meeting hours for this day
                // Note: We use DailyLogService to get meetings. 
                // This fetches from `daily_logs.json`.
                // If Stats JSON also had mtg, we summed it in Step 1.
                // But typically ArchiveService doesn't set mtg in stats (as discussed).
                // So `est` and `act` come from Stats+Tasks. `mtg` comes from DailyLogs+Stats(0).
                // This seems correct for now.
                
                let mtg = self.daily_log_service.get_log(day).ok().flatten().map(|l| l.total_hours()).unwrap_or(0.0);
                
                // We shouldn't add `mtg` to `act` or `est` here, just pass it to HistoryStats.
                // Wait, `weekly_data` stores `(Tasks, Est, Act, Mtg)`.
                // In Step 1, we added stats.mtg to entry.3.
                // Here we ignore entry.3? No.
                // `entry` is (day_tasks, est, act, mtg_from_stats).
                // We should combine `mtg_from_stats` + `mtg_from_repo`.
                // `entry.3` has `mtg` from `stats_repo`.
                // `mtg` var has `mtg` from `daily_log_repo`.
                // Total mtg = entry.3 + mtg.
                
                let stats_mtg = weekly_data.get(&(year, week)).unwrap().get(&day).unwrap().3;
                let total_mtg = mtg + stats_mtg;

                week_est += est;
                week_act += act;
                week_mtg += total_mtg;
                
                daily_histories.push(DailyHistory {
                    date: day.format("%Y-%m-%d").to_string(),
                    day_of_week: day.format("%a").to_string(),
                    tasks: day_tasks,
                    stats: HistoryStats {
                        total_est_hours: est,
                        total_act_hours: act,
                        meeting_hours: total_mtg,
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
