use crate::model::task::{Task, TaskState};
use crate::model::stats::MonthlyStats;
use crate::repository::{TaskRepository, FileStatsRepository}; // Assuming generic Repo is hard, we use FileStatsRepo directly or trait? 
// For simplicity in this script-like service, we use concrete FileStatsRepo or define a trait if needed.
// But wait, TaskRepository is a trait.
// Let's use concrete FileStatsRepository for now as it's new.

use chrono::{Datelike, Utc, Duration, DateTime};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

pub struct ArchiveService<R: TaskRepository> {
    task_repo: R,
    stats_repo: FileStatsRepository,
    archive_dir: PathBuf,
}

impl<R: TaskRepository> ArchiveService<R> {
    pub fn new(task_repo: R, stats_repo: FileStatsRepository) -> Self {
        let mut archive_dir = dirs::home_dir().expect("Home dir not found");
        archive_dir.push(".todoism");
        archive_dir.push("archive");
        fs::create_dir_all(&archive_dir).unwrap(); // Ensure exists

        Self {
            task_repo,
            stats_repo,
            archive_dir,
        }
    }

    pub fn archive_old_tasks(&self, cutoff_days: i64) -> Result<usize> {
        let all_tasks = self.task_repo.list()?;
        let now = Utc::now();
        let cutoff_date = now - Duration::days(cutoff_days);

        let mut tasks_to_archive = Vec::new();
        let mut tasks_to_keep = Vec::new();

        for task in all_tasks {
            let should_archive = match &task.state {
                TaskState::Completed { completed_at, .. } => *completed_at < cutoff_date,
                TaskState::Deleted => task.created_at < cutoff_date, // Archive old deleted too? Sure.
                _ => false,
            };

            if should_archive {
                tasks_to_archive.push(task);
            } else {
                tasks_to_keep.push(task);
            }
        }

        if tasks_to_archive.is_empty() {
            return Ok(0);
        }

        // 1. Update Stats
        self.update_stats(&tasks_to_archive)?;

        // 2. Write to Archive Files
        self.write_to_archive(&tasks_to_archive)?;

        // 3. Update Task Repo (Delete archived)
        // Since repo doesn't have "bulk update", we might need to delete one by one or overwrite file.
        // If R is FileTaskRepository, it has `save_all`? No.
        // It has `create`, `update`, `delete`.
        // We can call `delete` for each.
        for task in &tasks_to_archive {
            self.task_repo.delete(&task.id)?;
        }

        Ok(tasks_to_archive.len())
    }

    fn update_stats(&self, tasks: &[Task]) -> Result<()> {
        // Group by Month
        let mut monthly_groups: HashMap<(i32, u32), MonthlyStats> = HashMap::new();

        for task in tasks {
            // Determine date for stats. For Completed, use completed_at. For Deleted, maybe created_at?
            // Stats usually track "Work Done". Deleted tasks usually have 0 work done (unless logged).
            // If they have logs, we should credit the logs to the date they happened?
            // COMPLEXITY: Real stats should aggregate TimeLogs by their specific dates.
            // Simplified MVP: Credit total Act/Est to the completion date.
            // User agreed to: "monthly json... est,act,meeting structured".
            // Let's stick to: Credit to Completed Date.
            
            if let TaskState::Completed { completed_at, actual, time_logs: _ } = &task.state {
                let local_dt = DateTime::<chrono::Local>::from(*completed_at);
                let date_str = local_dt.format("%Y-%m-%d").to_string();
                let year = local_dt.year();
                let month = local_dt.month();

                let stats = monthly_groups.entry((year, month))
                    .or_insert_with(|| self.stats_repo.get_stats(year, month).unwrap_or(MonthlyStats::new(year, month)));

                let est = crate::service::task_service::parse_est_hours(&task.estimate);
                
                // Act parsing (using days logic from DTO/History)
                // Note: In DTO/history we divide by 3600*8? 
                // Wait, previous logic: "input 1 = 8 hours". 
                // `parse_est_hours` returns HOURS (input * 8).
                // `act` input string is DAYS (e.g. "0.5").
                
                let act_hours = if let Some(act_str) = actual {
                    act_str.parse::<f64>().unwrap_or(0.0) * 8.0
                } else {
                    0.0 // Ignore logs for archived tasks to simplify? Or sum logs?
                    // Previous history logic summed logs if act is missing.
                    // For archive, let's assume act exists or 0.
                };
                
                // Mtg? Not stored in task. Mtg is in DailyLog. 
                // DailyLog is NOT archived here (it's separate).
                // So Stats JSON only holds Task Est/Act. 
                // Mtg will be read from DailyLogRepo (which should arguably be archived too, but maybe later).
                // For now, Stats JSON has `mtg` field but we might leave it 0 if we don't aggregate logs.
                // Or does `HistoryUseCase` read DailyLog for meetings? Yes.
                // So ArchiveService only handles Tasks.
                
                stats.add(date_str, est, act_hours, 0.0);
            }
        }

        for (_, stats) in monthly_groups {
            self.stats_repo.save_stats(&stats)?;
        }

        Ok(())
    }

    fn write_to_archive(&self, tasks: &[Task]) -> Result<()> {
        // Group by Month
        let mut file_map: HashMap<(i32, u32), Vec<&Task>> = HashMap::new();
        
        for task in tasks {
            let dt = match &task.state {
                TaskState::Completed { completed_at, .. } => *completed_at,
                TaskState::Deleted => task.created_at, // Sort of arbitrary
                _ => task.created_at,
            };
            let local = DateTime::<chrono::Local>::from(dt);
            file_map.entry((local.year(), local.month())).or_default().push(task);
        }

        for ((year, month), tasks) in file_map {
            let filename = format!("tasks_{:04}_{:02}.json", year, month);
            let path = self.archive_dir.join(filename);
            
            // Read existing if any
            let mut existing_tasks: Vec<Task> = if path.exists() {
                let content = fs::read_to_string(&path)?;
                serde_json::from_str(&content).unwrap_or_default()
            } else {
                Vec::new()
            };
            
            // Merge (avoid dupes? IDs should be unique. Just append)
            for t in tasks {
                existing_tasks.push(t.clone());
            }
            
            let content = serde_json::to_string_pretty(&existing_tasks)?;
            fs::write(path, content)?;
        }
        Ok(())
    }
}
