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
        let completed_tasks: Vec<_> = tasks.iter()
            .filter(|t| matches!(t.state, TaskState::Completed { .. }))
            .collect();

        // Pass 1: Place tasks in their completion slots (for listing) and distribute actual hours
        for task in &completed_tasks {
            if let TaskState::Completed { completed_at, actual_duration, time_logs } = &task.state {
                let local_dt: DateTime<Local> = DateTime::from(*completed_at);
                let date = local_dt.date_naive();
                let iso = local_dt.iso_week();
                let week_key = (iso.year(), iso.week());

                // Ensure the day entry exists for the completion date
                let day_entry_for_completion = weekly_data.entry(week_key).or_default().entry(date).or_default();
                day_entry_for_completion.0.push(TaskDto::from_entity((*task).clone(), 0.0));
                
                let est = parse_est_hours(&task.estimate);
                day_entry_for_completion.1 += est; // Add to Est total for that day
                
                // For Act hours:
                // If we have logs, we distribute them.
                // If no logs (legacy), we attribute actual_duration here.
                if time_logs.is_empty() {
                     if let Some(dur) = actual_duration {
                         day_entry_for_completion.2 += *dur as f64 / 3600.0;
                     }
                } else {
                    // Iterate logs and distribute to respective days/weeks
                    for log in time_logs {
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
