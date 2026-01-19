use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use serde_json;
use uuid::Uuid;

use crate::model::task::Task;
use crate::repository::traits::TaskRepository;

const DEFAULT_FILE_NAME: &str = "tasks.json";

#[derive(Clone)]
pub struct FileTaskRepository {
    file_path: PathBuf,
}

impl FileTaskRepository {
    pub fn new(base_dir: Option<PathBuf>) -> Result<Self> {
        let mut path = match base_dir {
            Some(dir) => dir,
            None => {
                // Determine the default data directory (e.g., ~/.config/todoism or ~/.todoism)
                // For now, let's use a simple .todoism in the home directory
                let home_dir = dirs::home_dir()
                    .ok_or_else(|| anyhow!("Could not determine home directory"))?;
                home_dir.join(".todoism")
            }
        };
        fs::create_dir_all(&path)?; // Ensure the directory exists
        path.push(DEFAULT_FILE_NAME);

        // Ensure the file itself exists; create if it doesn't
        if !path.exists() {
            File::create(&path)?;
            // Write an empty JSON array to initialize it
            let mut writer = BufWriter::new(File::create(&path)?);
            serde_json::to_writer_pretty(&mut writer, &Vec::<Task>::new())?;
            writer.flush()?;
        }

        Ok(FileTaskRepository { file_path: path })
    }

    fn read_tasks(&self) -> Result<Vec<Task>> {
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let tasks = serde_json::from_reader(reader)?;
        Ok(tasks)
    }

    fn write_tasks(&self, tasks: &[Task]) -> Result<()> {
        let file = File::create(&self.file_path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, tasks)?;
        writer.flush()?;
        Ok(())
    }
}

impl TaskRepository for FileTaskRepository {
    fn create(&self, task: Task) -> Result<Task> {
        let mut tasks = self.read_tasks()?;
        tasks.push(task.clone());
        self.write_tasks(&tasks)?;
        Ok(task)
    }

    fn list(&self) -> Result<Vec<Task>> {
        self.read_tasks()
    }

    fn update(&self, task: &Task) -> Result<()> {
        let mut tasks = self.read_tasks()?;
        if let Some(pos) = tasks.iter().position(|t| t.id == task.id) {
            tasks[pos] = task.clone();
            self.write_tasks(&tasks)?;
            Ok(())
        } else {
            Err(anyhow!("Task with ID {} not found", task.id))
        }
    }

    fn delete(&self, id: &Uuid) -> Result<()> {
        let mut tasks = self.read_tasks()?;
        let initial_len = tasks.len();
        tasks.retain(|t| t.id != *id);
        
        if tasks.len() == initial_len {
            return Err(anyhow!("Task with ID {} not found", id));
        }

        self.write_tasks(&tasks)?;
        Ok(())
    }
}
