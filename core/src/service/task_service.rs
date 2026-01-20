use crate::model::task::{Task, Priority, TaskState};
use crate::repository::TaskRepository;
use crate::time::parse_duration;
use crate::service::dto::TaskDto;
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
    pub repo: R, // Making repo public so UseCase can access it, or expose get_all methods. UseCases usually access Repos directly. 
                 // But HistoryUseCase currently takes &TaskService but I changed it to take &R. 
                 // Wait, I implemented HistoryUseCase to take &R. 
                 // So TaskService doesn't need to expose repo if UseCase gets repo instance separately. 
                 // OR TaskService exposes repo. Let's make it pub for now or just allow UseCase to have the repo reference passed in main.
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

// get_weekly_history, has_daily_log, add_daily_log removed
}

pub fn parse_est_hours(est_opt: &Option<String>) -> f64 {
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
