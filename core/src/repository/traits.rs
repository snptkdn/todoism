use crate::model::task::Task;
use anyhow::Result;
use uuid::Uuid;

pub trait TaskRepository {
    fn create(&self, task: Task) -> Result<Task>;
    fn list(&self) -> Result<Vec<Task>>;
    fn update(&self, task: &Task) -> Result<()>;
    fn delete(&self, id: &Uuid) -> Result<()>;
}
