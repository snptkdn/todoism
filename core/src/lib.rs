pub mod model;
pub mod repository;
pub mod input;
pub mod time;
pub mod service;

pub use model::task::{Task, Priority, Status};
pub use repository::{TaskRepository, FileTaskRepository};
pub use input::{parse_args, expand_key, ParsedInput};
pub use time::{parse_human_date, parse_duration};
pub use service::task_service::{TaskService, SortStrategy, calculate_score, sort_tasks};

pub fn greet() -> String {
    "Hello from Todoism Core!".to_string()
}