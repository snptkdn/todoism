
#[cfg(test)]
mod tests {
    use crate::usecase::history::HistoryUseCase;
    use crate::repository::{TaskRepository, DailyLogRepository};
    use crate::service::daily_log_service::DailyLogService;
    use crate::model::task::{Task, TaskState, TimeLog};
    use crate::model::daily_log::DailyLog;
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use anyhow::Result;

    struct MockTaskRepo {
        tasks: Vec<Task>,
    }

    impl TaskRepository for MockTaskRepo {
        fn create(&self, _task: Task) -> Result<Task> { unimplemented!() }
        fn get(&self, _id: &Uuid) -> Result<Task> { unimplemented!() }
        fn update(&self, _task: &Task) -> Result<()> { unimplemented!() }
        fn delete(&self, _id: &Uuid) -> Result<()> { unimplemented!() }
        fn list(&self) -> Result<Vec<Task>> { Ok(self.tasks.clone()) }
    }

    struct MockDailyLogRepo;
    impl DailyLogRepository for MockDailyLogRepo {
        fn get(&self, _date: chrono::NaiveDate) -> Result<Option<DailyLog>> { Ok(None) }
        fn upsert(&self, _log: DailyLog) -> Result<()> { Ok(()) }
    }

    #[test]
    fn test_get_weekly_history_split_days() {
        let mut task = Task::new("Split Task".to_string(), None);
        
        let now = Utc::now();
        let yesterday = now - Duration::days(1);
        
        let log1 = TimeLog {
            start: yesterday, 
            end: Some(yesterday + Duration::hours(1)),
        };
        let log2 = TimeLog {
            start: now,
            end: Some(now + Duration::hours(2)),
        };
        
        task.state = TaskState::Completed {
            completed_at: now,
            time_logs: vec![log1, log2],
            actual_duration: None,
        };

        let task_repo = MockTaskRepo { tasks: vec![task] };
        let log_repo = MockDailyLogRepo;
        let log_service = DailyLogService::new(log_repo);
        let history_usecase = HistoryUseCase::new(&task_repo, &log_service);

        let history = history_usecase.get_weekly_history().unwrap();
        
        // We expect to find the task listed.
        // And we expect stats for TODAY (2 hours) and YESTERDAY (1 hour).
        // Since get_weekly_history groups by weeks, we might get 1 week (if yesterday and today are same week) or 2 weeks.
        
        let mut found_yesterday = false;
        let mut found_today = false;
        
        for week in history {
            for day in week.days {
                if day.date == yesterday.format("%Y-%m-%d").to_string() {
                    assert_eq!(day.stats.total_act_hours, 1.0);
                    found_yesterday = true;
                }
                if day.date == now.format("%Y-%m-%d").to_string() {
                    assert_eq!(day.stats.total_act_hours, 2.0);
                    found_today = true;
                }
            }
        }
        
        assert!(found_yesterday, "Should have found stats for yesterday");
        assert!(found_today, "Should have found stats for today");
    }
}
