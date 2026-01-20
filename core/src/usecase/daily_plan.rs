use crate::repository::DailyLogRepository;
use crate::service::daily_log_service::DailyLogService;
use crate::service::dto::TaskDto;
use anyhow::Result;
use chrono::Local;

#[derive(Default, Clone, Copy)]
pub struct DailyPlanStats {
    pub total_capacity: f64,
    pub meeting_hours: f64,
    pub work_done_today: f64,
    pub remaining_active_capacity: f64,
}

pub struct DailyPlanUseCase<'a, L: DailyLogRepository> {
    daily_log_service: &'a DailyLogService<L>,
}

impl<'a, L: DailyLogRepository> DailyPlanUseCase<'a, L> {
    pub fn new(daily_log_service: &'a DailyLogService<L>) -> Self {
        Self {
            daily_log_service,
        }
    }

    pub fn apply_daily_plan(&self, tasks: &mut [TaskDto]) -> Result<DailyPlanStats> {
        let today = Local::now().date_naive();
        
        // 1. Get Meeting Hours
        let meeting_hours = self.daily_log_service.get_log(today)
            .ok().flatten()
            .map(|l| l.total_hours())
            .unwrap_or(0.0);

        // 2. Tasks are passed in

        // 3. Calculate Work Done Today (across ALL tasks, assuming caller passed all relevant tasks)
        let work_done_today: u64 = tasks.iter()
            .map(|t| t.today_accumulated_time)
            .sum();
        let work_done_hours = work_done_today as f64 / 3600.0;
        
        // 4. Calculate Capacity
        let total_capacity = 8.0;
        let effective_capacity = (total_capacity - meeting_hours).max(0.0);
        let remaining_active_capacity = (effective_capacity - work_done_hours).max(0.0);

        // 5. Calculate Fit for Pending Tasks
        for task in tasks.iter_mut() {
            if task.status == "Pending" && !task.is_tracking {
                if task.remaining_estimate > 0.0 && task.remaining_estimate <= remaining_active_capacity {
                    task.fit = Some(true);
                } else if task.remaining_estimate > 0.0 {
                    task.fit = Some(false);
                } else {
                    task.fit = None; // No estimate
                }
            } else {
                task.fit = None;
            }
        }

        Ok(DailyPlanStats {
            total_capacity,
            meeting_hours,
            work_done_today: work_done_hours,
            remaining_active_capacity,
        })
    }
}
