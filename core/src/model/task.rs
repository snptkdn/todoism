use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}

// Old Status enum is replaced by TaskState logic, 
// but we might keep a simple enum for sorting/filtering if needed, 
// or just rely on matching TaskState. 
// For DTOs we use strings or a simple enum.
// To keep things clean, we will remove the old Status enum 
// and define TaskState.

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending {
        #[serde(default)]
        time_logs: Vec<TimeLog>,
    },
    Completed {
        completed_at: DateTime<Utc>,
        #[serde(default)]
        time_logs: Vec<TimeLog>,
        // Backwards compatibility for old data that only has actual_duration
        #[serde(default)]
        actual_duration: Option<u64>,
    },
    Deleted,
}

impl Default for TaskState {
    fn default() -> Self {
        TaskState::Pending { time_logs: Vec::new() }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub priority: Priority,
    
    pub state: TaskState,
    
    pub due: Option<DateTime<Utc>>, 
    pub description: Option<String>,
    pub project: Option<String>,
    pub estimate: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TimeLog {
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
}

impl Task {
    pub fn new(name: String, due: Option<DateTime<Utc>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            priority: Priority::default(),
            state: TaskState::default(),
            due,
            description: None,
            project: None,
            estimate: None,
            created_at: Utc::now(),
        }
    }

    pub fn start_tracking(&mut self) {
        if let TaskState::Pending { time_logs } = &mut self.state {
            let is_tracking = time_logs.last().map(|log| log.end.is_none()).unwrap_or(false);
            if !is_tracking {
                time_logs.push(TimeLog {
                    start: Utc::now(),
                    end: None,
                });
            }
        }
    }

    pub fn stop_tracking(&mut self) {
        if let TaskState::Pending { time_logs } = &mut self.state {
             if let Some(last_log) = time_logs.last_mut() {
                if last_log.end.is_none() {
                    last_log.end = Some(Utc::now());
                }
            }
        }
    }

    pub fn is_tracking(&self) -> bool {
        if let TaskState::Pending { time_logs } = &self.state {
             time_logs.last().map(|log| log.end.is_none()).unwrap_or(false)
        } else {
            false
        }
    }

    pub fn complete(&mut self) {
        if let TaskState::Completed { .. } = self.state {
            return;
        }
        
        // Extract logs if Pending
        let logs = if let TaskState::Pending { time_logs } = &mut self.state {
            // Stop tracking first if running
            if let Some(last_log) = time_logs.last_mut() {
                if last_log.end.is_none() {
                    last_log.end = Some(Utc::now());
                }
            }
            std::mem::take(time_logs)
        } else {
            Vec::new()
        };

        self.state = TaskState::Completed {
            completed_at: Utc::now(),
            time_logs: logs,
            actual_duration: None, // No longer needed for new completions
        };
    }
    
    // Helper to revert completion or un-delete (simplistic implementation)
    pub fn reopen(&mut self) {
         if !matches!(self.state, TaskState::Pending { .. }) {
             // Reset to Pending with empty logs. 
             // History of previous completion is lost in this simple model, 
             // or we could decide to keep 'actual_duration' as a starting offset.
             // For now, simple reset.
             self.state = TaskState::default();
         }
    }

    pub fn delete(&mut self) {
        self.state = TaskState::Deleted;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_tracking_lifecycle() {
        let mut task = Task::new("Test Task".to_string(), None);

        // 1. Start tracking
        task.start_tracking();
        assert!(task.is_tracking());
        
        if let TaskState::Pending { time_logs } = &task.state {
             assert_eq!(time_logs.len(), 1);
             assert!(time_logs[0].end.is_none());
        } else {
            panic!("Task should be Pending");
        }

        // 2. Stop tracking
        task.stop_tracking();
        assert!(!task.is_tracking());
        
        if let TaskState::Pending { time_logs } = &task.state {
             assert!(time_logs[0].end.is_some());
        }

        // 3. Start again
        task.start_tracking();
        if let TaskState::Pending { time_logs } = &task.state {
             assert_eq!(time_logs.len(), 2);
        }
        
        // 4. Complete task (should auto-stop and switch state)
        task.complete();
        
        if let TaskState::Completed { time_logs, actual_duration, completed_at: _ } = &task.state {
            assert!(!time_logs.is_empty(), "Time logs should be preserved");
            assert_eq!(time_logs.len(), 2); 
            assert!(actual_duration.is_none(), "New completions should not set actual_duration");
        } else {
            panic!("Task should be Completed");
        }
    }
}
