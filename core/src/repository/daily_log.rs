use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use serde_json;
use crate::model::daily_log::DailyLog;

const DAILY_LOG_FILE_NAME: &str = "daily_logs.json";

// We can define a trait for it, or just use the struct directly if we don't need mocking yet.
// Since TaskRepository is a trait, let's follow the pattern but keep it simple for now. 
// We will define a trait in the traits module if needed, but given the plan, 
// let's make a specific FileDailyLogRepository first.

pub trait DailyLogRepository {
    fn get(&self, date: NaiveDate) -> Result<Option<DailyLog>>;
    fn upsert(&self, log: DailyLog) -> Result<()>;
}

pub struct FileDailyLogRepository {
    file_path: PathBuf,
}

impl FileDailyLogRepository {
    pub fn new(base_dir: Option<PathBuf>) -> Result<Self> {
        let mut path = match base_dir {
            Some(dir) => dir,
            None => {
                let home_dir = dirs::home_dir()
                    .ok_or_else(|| anyhow!("Could not determine home directory"))?;
                home_dir.join(".todoism")
            }
        };
        fs::create_dir_all(&path)?;
        path.push(DAILY_LOG_FILE_NAME);

        if !path.exists() {
            File::create(&path)?;
            let mut writer = BufWriter::new(File::create(&path)?);
            serde_json::to_writer_pretty(&mut writer, &Vec::<DailyLog>::new())?;
            writer.flush()?;
        }

        Ok(FileDailyLogRepository { file_path: path })
    }

    fn read_logs(&self) -> Result<Vec<DailyLog>> {
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let logs: Vec<DailyLog> = serde_json::from_reader(reader)?;
        Ok(logs)
    }

    fn write_logs(&self, logs: &[DailyLog]) -> Result<()> {
        let file = File::create(&self.file_path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, logs)?;
        writer.flush()?;
        Ok(())
    }
}

impl DailyLogRepository for FileDailyLogRepository {
    fn get(&self, date: NaiveDate) -> Result<Option<DailyLog>> {
        let logs = self.read_logs()?;
        Ok(logs.into_iter().find(|l| l.date == date))
    }

    fn upsert(&self, log: DailyLog) -> Result<()> {
        let mut logs = self.read_logs()?;
        if let Some(pos) = logs.iter().position(|l| l.date == log.date) {
            logs[pos] = log;
        } else {
            logs.push(log);
        }
        self.write_logs(&logs)?;
        Ok(())
    }
}
