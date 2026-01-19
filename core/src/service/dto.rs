use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::model::task::{Task, TaskState, Priority};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TaskDto {
    pub id: Uuid,
    pub name: String,
    pub priority: Priority,
    pub due: Option<DateTime<Utc>>,
    pub project: Option<String>,
    pub estimate: Option<String>,
    pub description: Option<String>,
    
    // Flattened state fields for UI
    pub status: String,      // "Pending", "Completed", "Deleted"
    pub is_tracking: bool,
    pub accumulated_time: u64, // In seconds. For Pending: sum of logs. For Completed: actual_duration.
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    
    // Score for sorting/display
    pub score: f64,
}

impl TaskDto {
    pub fn from_entity(task: Task, score: f64) -> Self {
        let (status_str, is_tracking, accumulated_time, completed_at) = match &task.state {
            TaskState::Pending { time_logs } => {
                let tracking = time_logs.last().map(|l| l.end.is_none()).unwrap_or(false);
                let mut total = 0;
                for log in time_logs {
                    let end = log.end.unwrap_or_else(Utc::now);
                    if let Ok(duration) = end.signed_duration_since(log.start).to_std() {
                        total += duration.as_secs();
                    }
                }
                ("Pending", tracking, total, None)
            },
            TaskState::Completed { completed_at, actual_duration } => {
                ("Completed", false, *actual_duration, Some(*completed_at))
            },
            TaskState::Deleted => {
                ("Deleted", false, 0, None)
            }
        };

        Self {
            id: task.id,
            name: task.name,
            priority: task.priority,
            due: task.due,
            project: task.project,
            estimate: task.estimate,
            description: task.description,
            status: status_str.to_string(),
            is_tracking,
            accumulated_time,
            created_at: task.created_at,
            completed_at,
            score,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct HistoryStats {
    pub total_est_hours: f64,
    pub total_act_hours: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DailyHistory {
    pub date: String, // YYYY-MM-DD
    pub day_of_week: String, // Mon, Tue...
    pub tasks: Vec<TaskDto>,
    pub stats: HistoryStats,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WeeklyHistory {
    pub year: i32,
    pub week: u32,
    pub days: Vec<DailyHistory>,
    pub stats: HistoryStats,
}
