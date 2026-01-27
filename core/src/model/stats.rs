use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DailyStats {
    pub est: f64,
    pub act: f64,
    pub mtg: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MonthlyStats {
    pub year: i32,
    pub month: u32,
    pub days: HashMap<String, DailyStats>, // Key: "YYYY-MM-DD"
}

impl MonthlyStats {
    pub fn new(year: i32, month: u32) -> Self {
        Self {
            year,
            month,
            days: HashMap::new(),
        }
    }

    pub fn add(&mut self, date: String, est: f64, act: f64, mtg: f64) {
        let entry = self.days.entry(date).or_default();
        entry.est += est;
        entry.act += act;
        entry.mtg += mtg;
    }
}
