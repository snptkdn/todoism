use crate::model::task::{Task, Priority, TaskState};
use crate::repository::TaskRepository;
use crate::time::parse_duration;
use crate::service::dto::{TaskDto, WeeklyHistory, DailyHistory, HistoryStats};
use chrono::{Utc, Datelike, Local, NaiveDate, DateTime};
use anyhow::Result;
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortStrategy {
    Urgency,
    Priority,
    DueDate,
}

impl Default for SortStrategy {
    fn default() -> Self {
        SortStrategy::Urgency
    }
}

// Coefficients
const COEFFICIENT_DUE: f64 = 12.0;
const COEFFICIENT_PRIORITY: f64 = 6.0;
const COEFFICIENT_AGE: f64 = 2.0;
const COEFFICIENT_ESTIMATE: f64 = 5.0;

pub struct TaskService<R: TaskRepository> {
    repo: R,
}

impl<R: TaskRepository> TaskService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub fn create_task(&self, task: Task) -> Result<TaskDto> {
        let created = self.repo.create(task)?;
        let score = calculate_score(&created, SortStrategy::Urgency);
        Ok(TaskDto::from_entity(created, score))
    }

    pub fn get_sorted_tasks(&self, strategy: SortStrategy) -> Result<Vec<TaskDto>> {
        let mut tasks = self.repo.list()?;
        sort_tasks(&mut tasks, strategy);
        
        // Convert to DTOs
        let dtos = tasks.into_iter().map(|t| {
            let score = calculate_score(&t, strategy);
            TaskDto::from_entity(t, score)
        }).collect();
        
        Ok(dtos)
    }

    pub fn get_task(&self, id: &Uuid) -> Result<Task> {
        self.repo.get(id)
    }

    pub fn update_task(&self, task: &Task) -> Result<()> {
        self.repo.update(task)
    }

    pub fn delete_task(&self, id: &Uuid) -> Result<()> {
        self.repo.delete(id)
    }
    
    // State management methods
    
    pub fn start_task(&self, id: &Uuid) -> Result<()> {
        let mut task = self.repo.get(id)?;
        task.start_tracking();
        self.repo.update(&task)
    }

    pub fn stop_task(&self, id: &Uuid) -> Result<()> {
        let mut task = self.repo.get(id)?;
        task.stop_tracking();
        self.repo.update(&task)
    }

    pub fn complete_task(&self, id: &Uuid) -> Result<()> {
        let mut task = self.repo.get(id)?;
        task.complete();
        self.repo.update(&task)
    }

    pub fn toggle_status(&self, id: &Uuid) -> Result<()> {
        let mut task = self.repo.get(id)?;
        if matches!(task.state, TaskState::Completed { .. }) {
             task.reopen();
        } else {
             task.complete();
        }
        self.repo.update(&task)
    }
    
    // Sort helper specifically for the service if needed externally, 
    // but better to use the standalone function.
    pub fn sort(tasks: &mut Vec<Task>, strategy: SortStrategy) {
        sort_tasks(tasks, strategy);
    }

    pub fn get_weekly_history(&self) -> Result<Vec<WeeklyHistory>> {
        let tasks = self.repo.list()?;
        let completed_tasks: Vec<&Task> = tasks.iter()
            .filter(|t| matches!(t.state, TaskState::Completed { .. }))
            .collect();

        // Group by ISO Week
        let mut tasks_by_week: HashMap<(i32, u32), Vec<&Task>> = HashMap::new();

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
            let mut tasks_by_day: HashMap<NaiveDate, Vec<&Task>> = HashMap::new();
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

            for day in sorted_days {
                let daily_tasks = tasks_by_day.get(&day).unwrap();
                let mut day_dtos = Vec::new();
                let mut day_est = 0.0;
                let mut day_act = 0.0;

                for task in daily_tasks {
                    let est_hours = parse_est_hours(&task.estimate);
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

                daily_histories.push(DailyHistory {
                    date: day.format("%Y-%m-%d").to_string(),
                    day_of_week: day.format("%a").to_string(),
                    tasks: day_dtos,
                    stats: HistoryStats {
                        total_est_hours: day_est,
                        total_act_hours: day_act,
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
                }
            });
        }
        
        Ok(history)
    }
}

fn parse_est_hours(est_opt: &Option<String>) -> f64 {
    est_opt.as_ref()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0)
}

// Standalone functions for pure logic

pub fn sort_tasks(tasks: &mut Vec<Task>, strategy: SortStrategy) {
    tasks.sort_by(|a, b| {
        let score_a = calculate_score(a, strategy);
        let score_b = calculate_score(b, strategy);
        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
    });
}

pub fn calculate_score(task: &Task, strategy: SortStrategy) -> f64 {
    match strategy {
        SortStrategy::Urgency => calculate_urgency(task),
        SortStrategy::Priority => calculate_priority_score(task),
        SortStrategy::DueDate => calculate_due_score(task),
    }
}

fn calculate_urgency(task: &Task) -> f64 {
    // Only pending tasks have urgency
    if !matches!(task.state, TaskState::Pending { .. }) {
        return -100.0;
    }

    let mut score = 0.0;
    let now = Utc::now();

    if let Some(due) = task.due {
        if due < now {
            score += COEFFICIENT_DUE * 2.0; 
        } else {
            let diff = due - now;
            let days = diff.num_days();
            if days < 7 {
                score += COEFFICIENT_DUE;
                score += (7.0 - days as f64) * 0.5; 
            } else if days < 14 {
                score += COEFFICIENT_DUE * 0.5;
            } else {
                score += COEFFICIENT_DUE * 0.2;
            }
        }
    }

    match task.priority {
        Priority::High => score += COEFFICIENT_PRIORITY,
        Priority::Medium => score += COEFFICIENT_PRIORITY * 0.5,
        Priority::Low => score += COEFFICIENT_PRIORITY * 0.1,
    }

    let age = now - task.created_at;
    let days_old = age.num_days();
    if days_old > 0 {
        let age_score = (days_old as f64 / 100.0) * COEFFICIENT_AGE;
        score += age_score.min(COEFFICIENT_AGE);
    }

    if let Some(est_str) = &task.estimate {
        if let Ok(duration) = parse_duration(est_str) {
            let minutes = duration.num_minutes();
            if minutes > 0 && minutes <= 30 {
                score += COEFFICIENT_ESTIMATE;
            } else if minutes <= 60 {
                score += COEFFICIENT_ESTIMATE * 0.5;
            } else if minutes <= 120 {
                score += COEFFICIENT_ESTIMATE * 0.2;
            }
        }
    }

    score
}

fn calculate_priority_score(task: &Task) -> f64 {
    match task.priority {
        Priority::High => 3.0,
        Priority::Medium => 2.0,
        Priority::Low => 1.0,
    }
}

fn calculate_due_score(task: &Task) -> f64 {
    if let Some(due) = task.due {
            -(due.timestamp() as f64)
    } else {
        f64::MIN 
    }
}
