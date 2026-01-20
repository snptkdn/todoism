use crate::model::daily_log::DailyLog;
use crate::repository::DailyLogRepository;
use anyhow::Result;
use chrono::NaiveDate;

pub struct DailyLogService<R: DailyLogRepository> {
    repo: R,
}

impl<R: DailyLogRepository> DailyLogService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub fn get_log(&self, date: NaiveDate) -> Result<Option<DailyLog>> {
        self.repo.get(date)
    }

    pub fn add_log(&self, date: NaiveDate, hours: f64) -> Result<()> {
        let log = DailyLog::new(date, hours);
        self.repo.upsert(log)
    }

    pub fn has_log(&self, date: NaiveDate) -> Result<bool> {
        Ok(self.repo.get(date)?.is_some())
    }
}
