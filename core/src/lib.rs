pub mod model;
pub mod repository;
pub mod input;
pub mod time;
pub mod service;
pub mod usecase;

pub use model::task::{Task, Priority, TaskState};
pub use repository::{TaskRepository, FileTaskRepository, FileDailyLogRepository};
pub use input::{parse_args, expand_key, ParsedInput};
pub use time::{parse_human_date, parse_duration};
pub use service::task_service::{TaskService, SortStrategy, calculate_score, sort_tasks};
pub use service::daily_log_service::DailyLogService;
pub use service::dto::TaskDto;

pub fn greet() -> String {
    "Hello from Todoism Core!".to_string()
}