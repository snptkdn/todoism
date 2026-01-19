pub mod model;
pub mod repository;
pub mod input;
pub mod time;

pub use model::task::{Task, Priority, Status};
pub use repository::{TaskRepository, FileTaskRepository};
pub use input::{parse_args, expand_key, ParsedInput};
pub use time::parse_human_date;

pub fn greet() -> String {
    "Hello from Todoism Core!".to_string()
}