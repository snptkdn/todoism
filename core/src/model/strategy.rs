use crate::model::task::{Task, Priority, Status};
use crate::time::parse_duration;
use chrono::{Utc, Duration};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortStrategy {
    Urgency,
    Priority,
    DueDate,
    // Add more as needed
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

impl Task {
    pub fn score(&self, strategy: SortStrategy) -> f64 {
        match strategy {
            SortStrategy::Urgency => self.calculate_urgency(),
            SortStrategy::Priority => self.calculate_priority_score(),
            SortStrategy::DueDate => self.calculate_due_score(),
        }
    }

    fn calculate_urgency(&self) -> f64 {
        if self.status != Status::Pending {
            return -100.0;
        }

        let mut score = 0.0;
        let now = Utc::now();

        // 1. Due Date
        if let Some(due) = self.due {
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

        // 2. Priority
        match self.priority {
            Priority::High => score += COEFFICIENT_PRIORITY,
            Priority::Medium => score += COEFFICIENT_PRIORITY * 0.5,
            Priority::Low => score += COEFFICIENT_PRIORITY * 0.1,
        }

        // 3. Age
        let age = now - self.created_at;
        let days_old = age.num_days();
        if days_old > 0 {
            let age_score = (days_old as f64 / 100.0) * COEFFICIENT_AGE;
            score += age_score.min(COEFFICIENT_AGE);
        }

        // 4. Estimate
        if let Some(est_str) = &self.estimate {
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
    
    fn calculate_priority_score(&self) -> f64 {
        match self.priority {
            Priority::High => 3.0,
            Priority::Medium => 2.0,
            Priority::Low => 1.0,
        }
    }

    fn calculate_due_score(&self) -> f64 {
        // Closer due date = Higher score
        if let Some(due) = self.due {
            let now = Utc::now();
            let diff = due - now;
             // Invert timestamp roughly? Or just negate.
             // Earlier date (smaller timestamp) should be higher.
             -(due.timestamp() as f64)
        } else {
            f64::MIN // No due date = lowest priority
        }
    }
}

pub fn sort_tasks(tasks: &mut Vec<Task>, strategy: SortStrategy) {
    tasks.sort_by(|a, b| {
        let score_a = a.score(strategy);
        let score_b = b.score(strategy);
        // Descending score
        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
    });
}
