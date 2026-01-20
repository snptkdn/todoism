use serde::{Deserialize, Serialize};
use chrono::NaiveDate;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Meeting {
    pub name: String,
    pub hours: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DailyLog {
    pub date: NaiveDate,
    pub meetings: Vec<Meeting>,
}

impl DailyLog {
    pub fn new(date: NaiveDate, hours: f64) -> Self {
        Self {
            date,
            meetings: vec![Meeting {
                name: "all".to_string(),
                hours,
            }],
        }
    }

    pub fn total_hours(&self) -> f64 {
        self.meetings.iter().map(|m| m.hours).sum()
    }
}
