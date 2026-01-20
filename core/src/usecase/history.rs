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
        let tasks = self.task_repo.list()?;
        let completed_tasks: Vec<_> = tasks.iter()
            .filter(|t| matches!(t.state, TaskState::Completed { .. }))
            .collect();

        // Group by ISO Week
        let mut tasks_by_week: HashMap<(i32, u32), Vec<&crate::model::task::Task>> = HashMap::new();

        for task in &completed_tasks {
            if let TaskState::Completed { completed_at, .. } = &task.state {
                let local_dt: DateTime<Local> = DateTime::from(*completed_at);
                let iso_week = local_dt.iso_week();
                let key = (iso_week.year(), iso_week.week());
                tasks_by_week.entry(key).or_default().push(task);
            }
        }

        let mut sorted_weeks: Vec<_> = tasks_by_week.keys().cloned().collect();
        sorted_weeks.sort_by(|a, b| b.cmp(a));

        let mut history = Vec::new();

        for (year, week) in sorted_weeks {
            let tasks_in_week = tasks_by_week.get(&(year, week)).unwrap();
            
            // Group by Day
            let mut tasks_by_day: HashMap<chrono::NaiveDate, Vec<&crate::model::task::Task>> = HashMap::new();
             for task in tasks_in_week {
                 if let TaskState::Completed { completed_at, .. } = &task.state {
                    let local_dt: DateTime<Local> = DateTime::from(*completed_at);
                    tasks_by_day.entry(local_dt.date_naive()).or_default().push(task);
                }
            }
            
            let mut sorted_days: Vec<_> = tasks_by_day.keys().cloned().collect();
            sorted_days.sort_by(|a, b| b.cmp(a));
            
            let mut daily_histories = Vec::new();
            let mut week_est_total = 0.0;
            let mut week_act_total = 0.0;
            let mut week_mtg_total = 0.0;

            for day in sorted_days {
                let daily_tasks = tasks_by_day.get(&day).unwrap();
                let mut day_dtos = Vec::new();
                let mut day_est = 0.0;
                let mut day_act = 0.0;
                
                // Get meeting hours
                let meeting_hours = self.daily_log_service.get_log(day).ok().flatten().map(|l| l.total_hours()).unwrap_or(0.0);

                for task in daily_tasks {
                    let est_hours = parse_est_hours(&task.estimate); // Need to handle this helper
                    let act_hours = if let TaskState::Completed { actual_duration, .. } = &task.state {
                        *actual_duration as f64 / 3600.0
                    } else {
                        0.0
                    };

                    day_est += est_hours;
                    day_act += act_hours;
                    
                    day_dtos.push(TaskDto::from_entity((*task).clone(), 0.0));
                }
                
                week_est_total += day_est;
                week_act_total += day_act;
                week_mtg_total += meeting_hours;

                daily_histories.push(DailyHistory {
                    date: day.format("%Y-%m-%d").to_string(),
                    day_of_week: day.format("%a").to_string(),
                    tasks: day_dtos,
                    stats: HistoryStats {
                        total_est_hours: day_est,
                        total_act_hours: day_act,
                        meeting_hours: meeting_hours,
                    }
                });
            }

            history.push(WeeklyHistory {
                year,
                week,
                days: daily_histories,
                stats: HistoryStats {
                    total_est_hours: week_est_total,
                    total_act_hours: week_act_total,
                    meeting_hours: week_mtg_total,
                }
            });
        }
        
        Ok(history)
    }
}
