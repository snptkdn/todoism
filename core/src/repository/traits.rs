use crate::model::task::Task;
use anyhow::Result;

pub trait TaskRepository {
    fn create(&self, task: Task) -> Result<Task>;
    fn list(&self) -> Result<Vec<Task>>;
    // 今後 update, delete, find_by_id などを追加
}
