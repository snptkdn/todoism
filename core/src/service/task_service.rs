use crate::model::task::{Task, Priority, Status};
use crate::repository::TaskRepository;
use crate::time::parse_duration;
use chrono::Utc;
use anyhow::Result;
use uuid::Uuid;

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

    pub fn create_task(&self, task: Task) -> Result<Task> {
        self.repo.create(task)
    }

    pub fn get_sorted_tasks(&self, strategy: SortStrategy) -> Result<Vec<Task>> {
        let mut tasks = self.repo.list()?;
        sort_tasks(&mut tasks, strategy);
        Ok(tasks)
    }

    pub fn update_task(&self, task: &Task) -> Result<()> {
        self.repo.update(task)
    }

    pub fn delete_task(&self, id: &Uuid) -> Result<()> {
        self.repo.delete(id)
    }
    
    // Sort helper specifically for the service if needed externally, 
    // but better to use the standalone function.
    pub fn sort(tasks: &mut Vec<Task>, strategy: SortStrategy) {
        sort_tasks(tasks, strategy);
    }
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
    if task.status != Status::Pending {
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
