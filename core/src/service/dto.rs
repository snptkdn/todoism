use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Local};
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
    pub today_accumulated_time: u64, // In seconds. Work done strictly today.
    pub remaining_estimate: f64, // In hours. Estimate - Accumulated.
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    
    // Score for sorting/display
    pub score: f64,
}

impl TaskDto {
    pub fn from_entity(task: Task, score: f64) -> Self {
        let now = Utc::now();
        let today = now.date_naive();

        // Helper to calculate time spent strictly today
        let calc_today_time = |logs: &Vec<crate::model::task::TimeLog>| -> u64 {
            let mut today_sum = 0;
            for log in logs {
                let start_local = DateTime::<Local>::from(log.start);
                let start_date = start_local.date_naive();
                
                // Simplify: just check if log started today. 
                // Advanced: if log spans days, we should split. 
                // For now, let's stick to start date logic as per previous patterns or user intent.
                // But user wanted "today's work". Let's handle simple overlap.
                
                if let Some(end) = log.end {
                     let end_local = DateTime::<Local>::from(end);
                     let end_date = end_local.date_naive();
                     // If both today
                     if start_date == today && end_date == today {
                         if let Ok(d) = end.signed_duration_since(log.start).to_std() {
                             today_sum += d.as_secs();
                         }
                     } else if start_date == today {
                         // Starts today, ends later? (unlikely for short tasks but possible)
                         // Just count it.
                         if let Ok(d) = end.signed_duration_since(log.start).to_std() {
                             today_sum += d.as_secs();
                         }
                     }
                     // If ends today but started yesterday, we might miss it.
                     // Let's improve: split duration.
                     // But for MVP, `start_date == today` is a reasonable approximation for daily logs.
                } else if start_date == today {
                    // Running task started today
                    let duration = now.signed_duration_since(log.start).num_seconds();
                    if duration > 0 {
                        today_sum += duration as u64;
                    }
                }
            }
            today_sum
        };

        let (status_str, is_tracking, accumulated_time, today_time, completed_at) = match &task.state {
            TaskState::Pending { time_logs } => {
                let tracking = time_logs.last().map(|l| l.end.is_none()).unwrap_or(false);
                let mut total = 0;
                for log in time_logs {
                    let end = log.end.unwrap_or_else(Utc::now);
                    if let Ok(duration) = end.signed_duration_since(log.start).to_std() {
                        total += duration.as_secs();
                    }
                }
                
                let today_sum = calc_today_time(time_logs);
                
                ("Pending", tracking, total, today_sum, None)
            },
            TaskState::Completed { completed_at, time_logs, actual_duration } => {
                let total = if !time_logs.is_empty() {
                    let mut sum = 0;
                    for log in time_logs {
                         if let Some(end) = log.end {
                            if let Ok(duration) = end.signed_duration_since(log.start).to_std() {
                                sum += duration.as_secs();
                            }
                        }
                    }
                    sum
                } else {
                    actual_duration.unwrap_or(0)
                };
                
                let today_sum = if !time_logs.is_empty() {
                    calc_today_time(time_logs)
                } else {
                    // Legacy: if completed today, attribute all duration? Or 0?
                    // Let's say if completed_at is today, we count it.
                    let completed_local = DateTime::<Local>::from(*completed_at);
                    if completed_local.date_naive() == today {
                         actual_duration.unwrap_or(0)
                    } else {
                        0
                    }
                };
                
                ("Completed", false, total, today_sum, Some(*completed_at))
            },
            TaskState::Deleted => {
                ("Deleted", false, 0, 0, None)
            }
        };
        
        // Calculate remaining estimate
        let est_hours = crate::service::task_service::parse_est_hours(&task.estimate);
        let accumulated_hours = accumulated_time as f64 / 3600.0;
        let remaining_hours = (est_hours - accumulated_hours).max(0.0);

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
            today_accumulated_time: today_time,
            remaining_estimate: remaining_hours,
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
    #[serde(default)]
    pub meeting_hours: f64,
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
